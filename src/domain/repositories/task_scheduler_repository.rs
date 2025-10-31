use async_trait::async_trait;

use crate::domain::entities::scheduled_task::ScheduledTask;

#[derive(Debug)]
pub enum SchedulerError {
    TaskNotFound,
    StorageError(String),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SchedulerError::TaskNotFound => write!(f, "Task not found"),
            SchedulerError::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[async_trait]
pub trait TaskSchedulerRepository: Send + Sync {
    /// Add a task to the scheduler
    async fn add_scheduled_task(&self, task: ScheduledTask) -> Result<(), SchedulerError>;

    /// Get the next pending task (without removing it)
    async fn peek_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError>;

    /// Remove and return the next pending task
    async fn pop_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError>;

    /// Remove a specific task by ID
    async fn remove_task(&self, task_id: u64) -> Result<(), SchedulerError>;

    /// Check if there are any pending tasks
    async fn has_pending_tasks(&self) -> Result<bool, SchedulerError>;
}
