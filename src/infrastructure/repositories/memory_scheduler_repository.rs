use async_trait::async_trait;
use std::collections::BinaryHeap;
use tokio::sync::{Mutex, broadcast};

use crate::domain::entities::scheduled_task::ScheduledTask;
use crate::domain::repositories::task_scheduler_repository::{
    SchedulerError, TaskSchedulerRepository,
};

/// In-memory implementation of TaskSchedulerRepository using a priority queue with lazy deletion
#[derive(Debug)]
pub struct MemorySchedulerRepository {
    // ScheduledTask already implements correct ordering (earliest times first, deleted tasks sink)
    tasks: Mutex<BinaryHeap<ScheduledTask>>,
    // Channel to notify scheduler when new tasks are added
    wakeup_sender: broadcast::Sender<()>,
}

// Configuration for lazy deletion cleanup
const CLEANUP_THRESHOLD_RATIO: f64 = 0.25; // Cleanup when 25% of tasks are deleted
const MIN_TASKS_FOR_CLEANUP: usize = 100;  // Don't cleanup unless we have at least 100 tasks

impl MemorySchedulerRepository {
    pub fn new() -> Self {
        let (wakeup_sender, _) = broadcast::channel(1);
        Self {
            tasks: Mutex::new(BinaryHeap::new()),
            wakeup_sender,
        }
    }

    /// Get a receiver for wake-up notifications
    pub fn subscribe_wakeup(&self) -> broadcast::Receiver<()> {
        self.wakeup_sender.subscribe()
    }

    /// Cleanup deleted tasks if threshold is exceeded (periodic maintenance)
    async fn cleanup_if_needed(&self, tasks: &mut std::collections::BinaryHeap<ScheduledTask>) {
        let total_count = tasks.len();
        
        if total_count < MIN_TASKS_FOR_CLEANUP {
            return; // Don't cleanup small queues
        }

        let deleted_count = tasks.iter().filter(|task| task.is_marked_for_deletion()).count();
        let deleted_ratio = deleted_count as f64 / total_count as f64;

        if deleted_ratio >= CLEANUP_THRESHOLD_RATIO {
            // Rebuild heap without deleted tasks
            let original_heap = std::mem::replace(tasks, BinaryHeap::new());
            let mut new_heap = BinaryHeap::new();
            
            for task in original_heap.into_vec() {
                if !task.is_marked_for_deletion() {
                    new_heap.push(task);
                }
            }
            
            *tasks = new_heap;
        }
    }
}

impl Default for MemorySchedulerRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskSchedulerRepository for MemorySchedulerRepository {
    async fn add_scheduled_task(&self, task: ScheduledTask) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        
        // Check if this task should interrupt current sleep (is earlier than current top)
        let should_interrupt = tasks.peek()
            .map(|current_top| task.scheduled_time < current_top.scheduled_time)
            .unwrap_or(true); // If queue is empty, definitely interrupt
        
        tasks.push(task.clone());
        
        // Drop the lock before sending notification
        drop(tasks);
        
        // Send wake-up signal if this task should interrupt current sleep
        if should_interrupt {
            let _ = self.wakeup_sender.send(()); // Ignore if no receivers
        }
        
        Ok(())
    }

    async fn peek_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        
        // Skip deleted tasks at the top of the queue
        loop {
            match tasks.peek() {
                Some(next_task) if next_task.is_marked_for_deletion() => {
                    // Remove deleted task from top of queue and continue
                    tasks.pop().unwrap();
                    continue;
                }
                Some(next_task) => {
                    // Found a valid (non-deleted) task
                    return Ok(Some(next_task.clone()));
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }

    async fn pop_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        
        // Skip deleted tasks and return the first valid one
        loop {
            match tasks.pop() {
                Some(popped_task) if popped_task.is_marked_for_deletion() => {
                    // Skip deleted task and continue
                    continue;
                }
                Some(popped_task) => {
                    return Ok(Some(popped_task));
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }

    async fn remove_task(&self, task_id: u64) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.lock().await;

        // Lazy deletion: Mark task as deleted instead of immediate removal
        // This is O(n) search but O(1) deletion, much faster than heap rebuild
        let original_heap = std::mem::replace(&mut *tasks, BinaryHeap::new());
        let mut new_heap = BinaryHeap::new();
        let mut found_and_marked = false;

        for mut task in original_heap.into_vec() {
            if task.task_id == task_id && !task.is_marked_for_deletion() {
                // Mark as deleted instead of removing
                task.mark_deleted();
                found_and_marked = true;
            }
            new_heap.push(task);
        }

        *tasks = new_heap;

        if !found_and_marked {
            return Err(SchedulerError::TaskNotFound);
        }

        // Check if cleanup is needed
        self.cleanup_if_needed(&mut tasks).await;
        
        Ok(())
    }

    async fn has_pending_tasks(&self) -> Result<bool, SchedulerError> {
        let tasks = self.tasks.lock().await;
        Ok(!tasks.is_empty())
    }
}
