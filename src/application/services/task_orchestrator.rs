use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::Recurrence;
use crate::domain::entities::scheduled_task::ScheduledTask;
use crate::domain::entities::task::{NotificationMethod, Task};
use crate::domain::repositories::task_scheduler_repository::TaskSchedulerRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskOrchestrator {
    task_service: Arc<TaskService>,
    task_scheduler: Arc<dyn TaskSchedulerRepository>,
    timezone_service: Arc<TimezoneService>,
}

impl TaskOrchestrator {
    pub fn new(
        task_service: Arc<TaskService>,
        task_scheduler: Arc<dyn TaskSchedulerRepository>,
        timezone_service: Arc<TimezoneService>,
    ) -> Self {
        Self {
            task_service,
            task_scheduler,
            timezone_service,
        }
    }

    // === TASK CREATION ORCHESTRATION ===

    pub async fn handle_add_task_modal(
        &self,
        user_id: u64,
        guild_id: u64,
        task_type: &str,
        title: String,
        description: String,
        notification_method: NotificationMethod,
        input_str: String,
    ) -> Result<u64, String> {
        let (scheduled_time, recurrence) = self
            .timezone_service
            .parse_task_input(&input_str, task_type, user_id)
            .await?;

        let task_id = match task_type {
            "single" => {
                self.create_and_schedule_single_task(
                    user_id,
                    guild_id,
                    title,
                    description,
                    scheduled_time.unwrap(),
                    notification_method,
                )
                .await?
            }
            "weekly" => {
                if let Some(Recurrence::Weekly { days, hour, minute }) = recurrence {
                    self.create_and_schedule_weekly_task(
                        user_id,
                        guild_id,
                        title,
                        description,
                        days,
                        hour,
                        minute,
                        notification_method,
                    )
                    .await?
                } else {
                    return Err("Invalid recurrence type".to_string());
                }
            }
            _ => return Err(format!("Unknown task type: {}", task_type)),
        };

        Ok(task_id)
    }

    pub async fn create_and_schedule_single_task(
        &self,
        user_id: u64,
        guild_id: u64,
        title: String,
        description: String,
        scheduled_time: chrono::DateTime<chrono::Utc>,
        notification_method: NotificationMethod,
    ) -> Result<u64, String> {
        // delegate to task service
        let task_id = self
            .task_service
            .create_single_task(
                user_id,
                guild_id,
                title,
                description,
                scheduled_time,
                notification_method,
            )
            .await?;

        self.schedule_existing_task(task_id).await?;

        Ok(task_id)
    }

    pub async fn create_and_schedule_weekly_task(
        &self,
        user_id: u64,
        guild_id: u64,
        title: String,
        description: String,
        days: Vec<chrono::Weekday>,
        hour: u8,
        minute: u8,
        notification_method: NotificationMethod,
    ) -> Result<u64, String> {
        // delegate to task service
        let task_id = self
            .task_service
            .create_weekly_task(
                user_id,
                guild_id,
                title,
                description,
                days,
                hour,
                minute,
                notification_method,
            )
            .await?;

        self.schedule_existing_task(task_id).await?;

        Ok(task_id)
    }

    // === POST-NOTIFICATION ORCHESTRATION ===

    /// Handle task after notification (remove single tasks / reschedule recurring tasks)
    pub async fn handle_post_notification_task(&self, task: &Task) -> Result<(), String> {
        if task.recurrence.is_none() {
            // Single task - only remove from repository (already popped from scheduler during notification)
            self.task_service
                .remove_user_task(task.id, task.user_id)
                .await?;
        } else {
            // recurring task (weekly) - reschedule for next occurrence
            if let Some(next_time) = task.next_occurrence() {
                self.task_service
                    .task_repo
                    .update_task_time(task.id, next_time)
                    .map_err(|e| {
                        format!("Failed to reschedule recurring task #{}: {}", task.id, e)
                    })?;

                if let Some(updated_task) = self.task_service.get_task_by_id(task.id).await {
                    let scheduled_task = ScheduledTask::new(task.id, next_time, &updated_task);
                    self.task_scheduler
                        .add_scheduled_task(scheduled_task)
                        .await
                        .map_err(|e| format!("Failed to reschedule task in scheduler: {:?}", e))?;
                } else {
                    return Err(format!("Updated task #{} not found in repository after rescheduling", task.id));
                }
            } else {
                return Err(format!(
                    "Could not determine next occurrence for recurring task #{} '{}'",
                    task.id, task.title
                ));
            }
        }
        Ok(())
    }

    // === SCHEDULING UTILITIES ===

    /// Load all existing tasks into the scheduler at startup
    pub async fn initialize_scheduler_with_existing_tasks(&self) -> Result<(), String> {

        
        let all_tasks = self.task_service.get_all_tasks_for_scheduling().await;
        

        for task in all_tasks {
            if let Some(scheduled_time) = task.scheduled_time {
                let scheduled_task = ScheduledTask::new(task.id, scheduled_time, &task);
                self.task_scheduler
                    .add_scheduled_task(scheduled_task)
                    .await
                    .map_err(|e| format!("Failed to schedule existing task #{}: {:?}", task.id, e))?;

            }
        }
        Ok(())
    }

    /// Get the next pending task from the scheduler (for priority queue scheduler)
    pub async fn peek_next_scheduled_task(&self) -> Result<Option<ScheduledTask>, crate::domain::repositories::task_scheduler_repository::SchedulerError> {
        self.task_scheduler.peek_next_task().await
    }

    /// Remove and return the next pending task from the scheduler
    pub async fn pop_next_scheduled_task(&self) -> Result<Option<ScheduledTask>, crate::domain::repositories::task_scheduler_repository::SchedulerError> {
        self.task_scheduler.pop_next_task().await
    }

    /// Add a scheduled task to the scheduler (used for retries)
    pub async fn add_scheduled_task(&self, task: ScheduledTask) -> Result<(), crate::domain::repositories::task_scheduler_repository::SchedulerError> {
        self.task_scheduler.add_scheduled_task(task).await
    }

    // === USER-INITIATED TASK REMOVAL ===

    /// Remove a task initiated by user (removes from both repository and scheduler)
    pub async fn remove_user_task(&self, task_id: u64, user_id: u64) -> Result<Option<crate::domain::entities::task::Task>, String> {


        // First remove from repository (includes permission check)
        let removed_task = self.task_service.remove_user_task(task_id, user_id).await?;

        if removed_task.is_some() {
            // Also remove from scheduler (lazy deletion)

            self.task_scheduler
                .remove_task(task_id)
                .await
                .map_err(|e| format!("Failed to remove task from scheduler: {:?}", e))?;


        }

        Ok(removed_task)
    }

    /// Remove all tasks for a user (removes from both repository and scheduler)
    pub async fn remove_all_user_tasks(&self, user_id: u64) -> Result<usize, String> {


        // Get all user tasks before removal to know which scheduler entries to remove
        let user_tasks = self.task_service.get_user_tasks(user_id).await;
        
        // Remove from repository
        let removed_count = self.task_service.remove_all_user_tasks(user_id).await?;

        // Remove each task from scheduler
        for task in user_tasks {
            self.task_scheduler
                .remove_task(task.id)
                .await
                .map_err(|e| format!("Failed to remove task #{} from scheduler: {:?}", task.id, e))?;
        }


        Ok(removed_count)
    }

    /// Get task by ID (delegated to task service)
    pub async fn get_task_by_id(&self, task_id: u64) -> Option<Task> {
        self.task_service.get_task_by_id(task_id).await
    }

    async fn schedule_existing_task(&self, task_id: u64) -> Result<(), String> {
        if let Some(task) = self.task_service.get_task_by_id(task_id).await {
            if let Some(scheduled_time) = task.scheduled_time {
                let scheduled_task = ScheduledTask::new(task_id, scheduled_time, &task);
                self.task_scheduler
                    .add_scheduled_task(scheduled_task)
                    .await
                    .map_err(|e| format!("Failed to schedule task: {:?}", e))?;

            }
        }
        Ok(())
    }

    // === EDIT ORCHESTRATION ===

    pub async fn get_task_for_editing(&self, task_id: u64, user_id: u64) -> Option<Task> {
        self.task_service.get_task_for_editing(task_id, user_id).await
    }

    pub async fn get_user_tasks_for_removal(
        &self,
        user_id: u64,
    ) -> Result<(Vec<crate::domain::entities::task::Task>, Vec<crate::domain::entities::task::Task>), String> {
        self.task_service.get_user_tasks_for_removal(user_id).await
    }

    pub async fn edit_and_reschedule_task(
        &self,
        task_id: u64,
        user_id: u64,
        new_title: Option<String>,
        new_description: Option<String>,
        new_datetime_input: Option<String>,
        is_weekly_task: bool,
    ) -> Result<Task, String> {


        // execute editing in taskservice
        let edited_task = self
            .task_service
            .edit_task(
                task_id,
                user_id,
                new_title,
                new_description,
                new_datetime_input,
                is_weekly_task,
                self.timezone_service.clone(),
            )
            .await?;



        // first remove old task from scheduler

        self.task_scheduler
            .remove_task(task_id)
            .await
            .map_err(|e| format!("Failed to remove old schedule: {:?}", e))?;

        // then add new version if it has a scheduled time
        if let Some(scheduled_time) = edited_task.scheduled_time {
            let scheduled_task = ScheduledTask::new(task_id, scheduled_time, &edited_task);
            self.task_scheduler
                .add_scheduled_task(scheduled_task)
                .await
                .map_err(|e| format!("Failed to reschedule: {:?}", e))?;
        } else {
            println!("[ORCHESTRATOR] Edited task #{} has no scheduled_time, not adding to scheduler", task_id);
        }

        Ok(edited_task)
    }
}
