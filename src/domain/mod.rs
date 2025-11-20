pub mod entities;
pub mod repositories;
pub mod value_objects;

pub use entities::task::{NotificationMethod, Recurrence, Task, WeeklyRecurrenceData, EveryXDaysRecurrenceData};
// Re-exports for scheduler components - used via complex trait bounds
#[allow(unused_imports)]
pub use entities::scheduled_task::ScheduledTask;
#[allow(unused_imports)]
pub use repositories::task_scheduler_repository::{TaskSchedulerRepository, SchedulerError};