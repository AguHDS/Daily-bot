use async_trait::async_trait;
use std::collections::BinaryHeap;
use tokio::sync::Mutex;

use crate::domain::entities::scheduled_task::ScheduledTask;
use crate::domain::repositories::task_scheduler_repository::{
    SchedulerError, TaskSchedulerRepository,
};
use std::cmp::Reverse;

/// In-memory implementation of TaskSchedulerRepository using a priority queue
#[derive(Debug)]
pub struct MemorySchedulerRepository {
    // reverse so that BinaryHeap (max-heap) is combined as min-heap
    tasks: Mutex<BinaryHeap<Reverse<ScheduledTask>>>,
}

impl MemorySchedulerRepository {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(BinaryHeap::new()),
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
        tasks.push(Reverse(task));
        Ok(())
    }

    async fn peek_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.peek().map(|reverse_task| reverse_task.0.clone()))
    }

    async fn pop_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let mut tasks = self.tasks.lock().await;
        Ok(tasks.pop().map(|reverse_task| reverse_task.0))
    }

    async fn remove_task(&self, task_id: u64) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.lock().await;

        // search task by ID and remove it
        let original_heap = std::mem::replace(&mut *tasks, BinaryHeap::new());
        let mut new_heap = BinaryHeap::new();

        for reverse_task in original_heap.into_vec() {
            if reverse_task.0.task_id != task_id {
                new_heap.push(reverse_task);
            }
        }

        *tasks = new_heap;
        Ok(())
    }

    async fn has_pending_tasks(&self) -> Result<bool, SchedulerError> {
        let tasks = self.tasks.lock().await;
        Ok(!tasks.is_empty())
    }
}
