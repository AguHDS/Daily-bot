use chrono::{DateTime, Utc};
use std::cmp::Ordering;

/// Entity representing a task scheduled for notification
/// Used by the priority queue scheduler for efficient task management
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task_id: u64,
    pub scheduled_time: DateTime<Utc>,
    pub user_id: u64,
    pub guild_id: u64,
    pub title: String,
    pub notification_method: crate::domain::entities::task::NotificationMethod,
    pub is_recurring: bool,
}

impl ScheduledTask {
    pub fn new(
        task_id: u64,
        scheduled_time: DateTime<Utc>,
        task: &crate::domain::entities::task::Task,
    ) -> Self {
        Self {
            task_id,
            scheduled_time,
            user_id: task.user_id,
            guild_id: task.guild_id,
            title: task.title.clone(),
            notification_method: task.notification_method.clone(),
            is_recurring: task.recurrence.is_some(),
        }
    }
}

// implement ordering for priority queue (earliest times have highest priority)
impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order so earliest times come first in max-heap
        other.scheduled_time.cmp(&self.scheduled_time)
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for ScheduledTask {}
