pub mod config_repository;
pub mod task_repository;
pub mod user_preferences_repository;
pub mod task_scheduler_repository;

pub use config_repository::ConfigRepository;
pub use task_repository::TaskRepository;
pub use user_preferences_repository::UserPreferencesRepository;
// Scheduler components - used in Arc<dyn Trait> and error handling
#[allow(unused_imports)]
pub use task_scheduler_repository::{TaskSchedulerRepository, SchedulerError};