use crate::application::domain::task::{Recurrence, Task};
use chrono::{DateTime, Utc};

pub trait TaskRepository: Send + Sync {
    fn add_task(&self, task: Task) -> Result<u64, String>;

    fn edit_task(
        &self,
        task_id: u64,
        new_message: Option<String>,
        new_scheduled_time: Option<DateTime<Utc>>,
        new_recurrence: Option<Recurrence>,
    ) -> Result<Task, String>;

    fn remove_task(&self, task_id: u64) -> bool;

    fn remove_all_by_user(&self, user_id: u64) -> usize;

    fn list_tasks(&self) -> Vec<Task>;

    /// Updates only the scheduled time of a task (used for recurring weekly task)
    fn update_task_time(&self, task_id: u64, new_time: DateTime<Utc>) -> Result<(), String>;
}
