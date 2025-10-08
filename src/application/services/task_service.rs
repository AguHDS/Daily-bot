use crate::application::services::notification_service::NotificationService;
use crate::domain::entities::task::{NotificationMethod, Recurrence, Task};
use crate::domain::repositories::{ConfigRepository, TaskRepository};
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};
use std::sync::Arc;

#[derive(Clone)]
pub struct TaskService {
    task_repo: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    config_repo: Arc<dyn ConfigRepository>,
    #[allow(dead_code)]
    notification_service: Arc<NotificationService>,
}

impl TaskService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        config_repo: Arc<dyn ConfigRepository>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            task_repo,
            config_repo,
            notification_service,
        }
    }

    // === SCHEDULER BUSINESS LOGIC ===

    /// Get all tasks for scheduling (no user filtering)
    pub async fn get_all_tasks_for_scheduling(&self) -> Vec<Task> {
        self.task_repo.list_tasks()
    }

    /// Handle task after notification (remove single tasks / reschedule recurring tasks)
    pub async fn handle_post_notification_task(&self, task: &Task) -> Result<(), String> {
        if task.recurrence.is_none() {
            // Single task - remove after notification
            let removed = self.task_repo.remove_task(task.id);
            if removed {
                println!("✅ Single task #{} removed after notification", task.id);
            }
            Ok(())
        } else {
            // Recurring task - reschedule for next occurrence
            if let Some(next_time) = task.next_occurrence() {
                match self.task_repo.update_task_time(task.id, next_time) {
                    Ok(_) => {
                        println!(
                            "♻️ Recurring task #{} rescheduled for {}",
                            task.id, next_time
                        );
                        Ok(())
                    }
                    Err(err) => Err(format!(
                        "Failed to reschedule recurring task #{}: {}",
                        task.id, err
                    )),
                }
            } else {
                Err(format!(
                    "Could not determine next occurrence for recurring task #{}",
                    task.id
                ))
            }
        }
    }

    // === TASK CREATION BUSINESS LOGIC ===

    pub async fn create_single_task(
        &self,
        user_id: u64,
        guild_id: u64,
        message: String,
        scheduled_time: DateTime<Utc>,
        notification_method: NotificationMethod,
    ) -> Result<u64, String> {
        // Validaciones de negocio
        if scheduled_time < Utc::now() {
            return Err("Cannot create a task in the past".to_string());
        }

        if message.trim().is_empty() {
            return Err("Task message cannot be empty".to_string());
        }

        // Crear entidad
        let task = Task::new(
            0, // ID se asignará en el repositorio
            user_id,
            guild_id,
            message,
            Some(scheduled_time),
            None, // No recurrence para tarea única
            notification_method,
            None,
        );

        // Persistir
        self.task_repo.add_task(task)
    }

    pub async fn create_weekly_task(
        &self,
        user_id: u64,
        guild_id: u64,
        message: String,
        days: Vec<Weekday>,
        hour: u8,
        minute: u8,
        notification_method: NotificationMethod,
    ) -> Result<u64, String> {
        // Validaciones
        if message.trim().is_empty() {
            return Err("Task message cannot be empty".to_string());
        }

        if days.is_empty() {
            return Err("At least one day must be specified for weekly task".to_string());
        }

        if hour > 23 || minute > 59 {
            return Err("Invalid time specified".to_string());
        }

        // Calcular primera ocurrencia
        let first_time = self
            .calculate_first_occurrence(&days, hour, minute)
            .ok_or("Could not calculate first occurrence".to_string())?;

        if first_time < Utc::now() {
            return Err("Cannot create a weekly task in the past".to_string());
        }

        // Crear entidad
        let recurrence = Some(Recurrence::Weekly { days, hour, minute });
        let task = Task::new(
            0,
            user_id,
            guild_id,
            message,
            Some(first_time),
            recurrence,
            notification_method,
            None,
        );

        // Persistir
        self.task_repo.add_task(task)
    }

    fn calculate_first_occurrence(
        &self,
        days: &[Weekday],
        hour: u8,
        minute: u8,
    ) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        let mut candidate = now;

        // Buscar en los próximos 7 días
        for _ in 0..7 {
            candidate = candidate + Duration::days(1);
            if days.contains(&candidate.weekday()) {
                return candidate
                    .with_hour(hour as u32)
                    .and_then(|t| t.with_minute(minute as u32))
                    .and_then(|t| t.with_second(0))
                    .and_then(|t| t.with_nanosecond(0));
            }
        }
        None
    }

    pub async fn get_user_tasks(&self, user_id: u64) -> Vec<Task> {
        self.task_repo
            .list_tasks()
            .into_iter()
            .filter(|task| task.user_id == user_id)
            .collect()
    }

    // === REMOVE TASK BUSINESS LOGIC ===

    pub async fn remove_user_task(&self, task_id: u64, user_id: u64) -> Result<bool, String> {
        // Verificar que la tarea pertenece al usuario antes de eliminar
        let tasks = self.task_repo.list_tasks();
        if let Some(task) = tasks.into_iter().find(|t| t.id == task_id) {
            if task.user_id == user_id {
                let removed = self.task_repo.remove_task(task_id);
                return Ok(removed);
            } else {
                return Err("You don't have permission to delete this task".to_string());
            }
        }
        Ok(false)
    }

    pub async fn remove_all_user_tasks(&self, user_id: u64) -> Result<usize, String> {
        let count = self.task_repo.remove_all_by_user(user_id);
        Ok(count)
    }

    pub async fn get_user_tasks_for_removal(
        &self,
        user_id: u64,
    ) -> Result<(Vec<Task>, Vec<Task>), String> {
        let tasks = self.get_user_tasks(user_id).await;

        if tasks.is_empty() {
            return Err("You don't have any task to delete".to_string());
        }

        let single_tasks: Vec<Task> = tasks
            .iter()
            .filter(|t| t.recurrence.is_none())
            .cloned()
            .collect();

        let weekly_tasks: Vec<Task> = tasks
            .iter()
            .filter(|t| t.recurrence.is_some())
            .cloned()
            .collect();

        Ok((single_tasks, weekly_tasks))
    }

    // === LIST TASKS BUSINESS LOGIC ===

    pub async fn get_user_tasks_formatted(&self, user_id: u64) -> String {
        let tasks = self.get_user_tasks(user_id).await;

        if tasks.is_empty() {
            return "You don't have any tasks yet!".to_string();
        }

        // Separar tareas simples y recurrentes
        let (single_tasks, recurrent_tasks) = self.separate_tasks_by_type(&tasks);

        let mut content = String::from("📝 **Your tasks:**\n\n");

        // Mostrar tareas simples primero
        if !single_tasks.is_empty() {
            content.push_str("**Single Tasks:**\n");
            for task in &single_tasks {
                let scheduled_str = task
                    .scheduled_time
                    .map_or("Not scheduled".to_string(), |dt| {
                        dt.format("%Y-%m-%d at %H:%M").to_string()
                    });
                content.push_str(&format!(
                    "**#{}**: {} — {}\n",
                    task.id, task.message, scheduled_str
                ));
            }
            content.push('\n');
        }

        // Mostrar tareas recurrentes
        if !recurrent_tasks.is_empty() {
            content.push_str("**Recurrent Tasks:**\n");
            for task in &recurrent_tasks {
                let recurrence_str = self.format_recurrence_for_display(&task.recurrence);
                content.push_str(&format!(
                    "**#{}**: {} — {}\n",
                    task.id, task.message, recurrence_str
                ));
            }
        }

        content
    }

    fn separate_tasks_by_type<'a>(&self, tasks: &'a [Task]) -> (Vec<&'a Task>, Vec<&'a Task>) {
        let mut single_tasks: Vec<&'a Task> =
            tasks.iter().filter(|t| t.recurrence.is_none()).collect();

        let mut recurrent_tasks: Vec<&'a Task> =
            tasks.iter().filter(|t| t.recurrence.is_some()).collect();

        single_tasks.sort_by_key(|t| t.id);
        recurrent_tasks.sort_by_key(|t| t.id);

        (single_tasks, recurrent_tasks)
    }
    fn format_recurrence_for_display(&self, recurrence: &Option<Recurrence>) -> String {
        match recurrence {
            Some(Recurrence::Weekly { days, hour, minute }) => {
                let days_str: Vec<String> = days.iter().map(|d| format!("{:?}", d)).collect();
                format!(
                    "Weekly on {} at {:02}:{:02}",
                    days_str.join(", "),
                    hour,
                    minute
                )
            }
            Some(Recurrence::EveryXDays {
                interval,
                hour,
                minute,
            }) => {
                format!("Every {} days at {:02}:{:02}", interval, hour, minute)
            }
            None => "Unknown recurrence".to_string(),
        }
    }

    // === EDIT TASK BUSINESS LOGIC ===

    pub async fn get_user_tasks_for_editing(&self, user_id: u64) -> (Vec<Task>, Vec<Task>) {
        let tasks = self.task_repo.list_tasks();
        let user_tasks: Vec<Task> = tasks.into_iter().filter(|t| t.user_id == user_id).collect();

        let single_tasks: Vec<Task> = user_tasks
            .iter()
            .filter(|t| t.recurrence.is_none())
            .cloned()
            .collect();

        let weekly_tasks: Vec<Task> = user_tasks
            .iter()
            .filter(|t| t.recurrence.is_some())
            .cloned()
            .collect();

        (single_tasks, weekly_tasks)
    }

    pub async fn get_task_for_editing(&self, task_id: u64, user_id: u64) -> Option<Task> {
        self.task_repo
            .list_tasks()
            .into_iter()
            .find(|t| t.id == task_id && t.user_id == user_id)
    }

    pub async fn edit_task(
        &self,
        task_id: u64,
        user_id: u64,
        new_message: Option<String>,
        new_datetime_input: Option<String>,
        is_weekly_task: bool,
    ) -> Result<Task, String> {
        // Verificar que la tarea existe y pertenece al usuario
        let current_task = self
            .get_task_for_editing(task_id, user_id)
            .await
            .ok_or_else(|| "Task not found or you don't have permission to edit it".to_string())?;

        let (new_scheduled_time, new_recurrence) = if let Some(datetime_input) = new_datetime_input
        {
            if is_weekly_task {
                let (days, hour, minute) = Self::parse_weekly_task_input(&datetime_input)?;
                let first_time = self
                    .calculate_first_occurrence(&days, hour, minute)
                    .ok_or("Could not calculate first occurrence".to_string())?;

                if first_time < Utc::now() {
                    return Err("Cannot set a weekly task in the past".to_string());
                }

                (
                    Some(first_time),
                    Some(Recurrence::Weekly { days, hour, minute }),
                )
            } else {
                let scheduled_time = Self::parse_single_task_input(&datetime_input)?;

                if scheduled_time < Utc::now() {
                    return Err("Cannot set a date in the past".to_string());
                }

                (Some(scheduled_time), None)
            }
        } else {
            (current_task.scheduled_time, current_task.recurrence)
        };

        // Validar que el mensaje no esté vacío si se proporciona uno nuevo
        if let Some(ref message) = new_message {
            if message.trim().is_empty() {
                return Err("Task title cannot be empty".to_string());
            }
        }

        self.task_repo.edit_task(
            task_id,
            new_message,
            new_scheduled_time,
            new_recurrence,
            None, // notification_method remains unchanged
        )
    }

    pub fn format_task_for_display(&self, task: &Task) -> String {
        if let Some(Recurrence::Weekly { days, hour, minute }) = &task.recurrence {
            let days_str = days
                .iter()
                .map(|d| format!("{:?}", d))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "#{}: {} (Weekly on {} at {:02}:{:02})",
                task.id, task.message, days_str, hour, minute
            )
        } else if let Some(dt) = task.scheduled_time {
            format!(
                "#{}: {} (Single on {})",
                task.id,
                task.message,
                dt.format("%Y-%m-%d %H:%M")
            )
        } else {
            format!("#{}: {}", task.id, task.message)
        }
    }

    pub fn get_datetime_placeholder(&self, task: &Task) -> String {
        if task.recurrence.is_some() {
            "Enter days and hour (Mon,Wed,Fri 14:00)".to_string()
        } else {
            task.scheduled_time
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "YYYY-MM-DD HH:MM".to_string())
        }
    }

    // === COMMAND HANDLERS (para interaction_handlers) ===

    pub async fn handle_add_task_modal(
        &self,
        user_id: u64,
        guild_id: u64,
        task_type: &str,
        message: String,
        notification_method: NotificationMethod,
        input_str: String,
    ) -> Result<u64, String> {
        match task_type {
            "single" => {
                let scheduled_time = Self::parse_single_task_input(&input_str)?;
                self.create_single_task(
                    user_id,
                    guild_id,
                    message,
                    scheduled_time,
                    notification_method,
                )
                .await
            }
            "weekly" => {
                let (days, hour, minute) = Self::parse_weekly_task_input(&input_str)?;
                self.create_weekly_task(
                    user_id,
                    guild_id,
                    message,
                    days,
                    hour,
                    minute,
                    notification_method,
                )
                .await
            }
            _ => Err(format!("Unknown task type: {}", task_type)),
        }
    }

    // === PARSING UTILITIES ===

    fn parse_single_task_input(input_str: &str) -> Result<DateTime<Utc>, String> {
        use chrono::NaiveDateTime;

        let naive_dt = NaiveDateTime::parse_from_str(input_str, "%Y-%m-%d %H:%M")
            .map_err(|_| "Failed to parse date/time. Use format: YYYY-MM-DD HH:MM".to_string())?;

        Ok(Utc.from_utc_datetime(&naive_dt))
    }

    fn parse_weekly_task_input(input_str: &str) -> Result<(Vec<Weekday>, u8, u8), String> {
        use crate::application::commands::utils::parse_weekly_input;

        let (days, hour, minute, _) = parse_weekly_input(input_str)
            .map_err(|e| format!("Failed to parse weekly input: {}", e))?;

        Ok((days, hour, minute))
    }
}
