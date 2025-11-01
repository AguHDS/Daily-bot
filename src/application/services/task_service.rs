use crate::application::services::notification_service::NotificationService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::entities::task::{NotificationMethod, Recurrence, Task};
use crate::domain::repositories::{ConfigRepository, TaskRepository};
use crate::domain::value_objects::weekday_format::WeekdayFormat;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use serenity::model::colour::Color;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone)]
pub struct TaskService {
    pub(crate) task_repo: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    config_repo: Arc<dyn ConfigRepository>,
    #[allow(dead_code)]
    notification_service: Arc<NotificationService>,
    timezone_service: Arc<TimezoneService>,
}

impl TaskService {
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        config_repo: Arc<dyn ConfigRepository>,
        notification_service: Arc<NotificationService>,
        timezone_service: Arc<TimezoneService>,
    ) -> Self {
        Self {
            task_repo,
            config_repo,
            notification_service,
            timezone_service,
        }
    }

    // === TASK CREATION BUSINESS LOGIC ===

    pub async fn create_single_task(
        &self,
        user_id: u64,
        guild_id: u64,
        title: String,
        description: String,
        scheduled_time: DateTime<Utc>,
        notification_method: NotificationMethod,
        mention: Option<String>,
    ) -> Result<u64, String> {
        if scheduled_time < Utc::now() {
            return Err("Cannot create a task in the past".to_string());
        }

        if title.trim().is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        let task = Task::new(
            0, // id is assigned in the repo
            user_id,
            guild_id,
            title,
            Some(description),
            Some(scheduled_time),
            None,
            notification_method,
            None,
            mention,
        );

        // persist
        self.task_repo.add_task(task)
    }

    pub async fn create_weekly_task(
        &self,
        user_id: u64,
        guild_id: u64,
        title: String,
        description: String,
        days: Vec<Weekday>,
        hour: u8,
        minute: u8,
        notification_method: NotificationMethod,
        mention: Option<String>,
    ) -> Result<u64, String> {
        if title.trim().is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        if days.is_empty() {
            return Err("At least one day must be specified for weekly task".to_string());
        }

        if hour > 23 || minute > 59 {
            return Err("Invalid time specified".to_string());
        }

        // put first occurrence
        let first_time = self
            .calculate_first_occurrence(&days, hour, minute)
            .ok_or("Could not calculate first occurrence".to_string())?;

        if first_time < Utc::now() {
            return Err("Cannot create a weekly task in the past".to_string());
        }

        // create entity
        let recurrence = Some(Recurrence::Weekly { days, hour, minute });
        let task = Task::new(
            0,
            user_id,
            guild_id,
            title,
            Some(description),
            Some(first_time),
            recurrence,
            notification_method,
            None,
            mention,
        );

        // persist
        self.task_repo.add_task(task)
    }

    fn calculate_first_occurrence(
        &self,
        days: &[Weekday],
        hour: u8,
        minute: u8,
    ) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        // create "today at the specified time"
        let today_at_time = now
            .with_hour(hour as u32)
            .and_then(|t| t.with_minute(minute as u32))
            .and_then(|t| t.with_second(0))
            .unwrap();

        let mut candidate = today_at_time;

        // if today's time has already passed, start from tomorrow
        if candidate <= now {
            candidate = candidate + Duration::days(1);
        }

        // find the next matching day
        for _ in 0..7 {
            if days.contains(&candidate.weekday()) {
                return Some(candidate);
            }
            candidate = candidate + Duration::days(1);
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

    pub async fn remove_user_task(
        &self,
        task_id: u64,
        user_id: u64,
    ) -> Result<Option<Task>, String> {
        // verify that the task belongs to the user
        let tasks = self.task_repo.list_tasks();
        if let Some(task) = tasks.into_iter().find(|t| t.id == task_id) {
            if task.user_id == user_id {
                let removed = self.task_repo.remove_task(task_id);
                if removed {
                    return Ok(Some(task));
                } else {
                    return Ok(None);
                }
            } else {
                return Err("You don't have permission to delete this task".to_string());
            }
        }
        Ok(None)
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

    // Used for correct time formatting
    fn format_recurrence_for_display_with_timezone(
        &self,
        recurrence: &Option<Recurrence>,
        timezone_service: &TimezoneService,
        user_timezone: &str,
    ) -> String {
        match recurrence {
            Some(Recurrence::Weekly { days, hour, minute }) => {
                // format the days of the week
                let days_str = days
                    .iter()
                    .map(|d| d.to_short_en().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                // convert UTC to local time
                let utc_time = Utc::now()
                    .with_hour(*hour as u32)
                    .and_then(|t| t.with_minute(*minute as u32))
                    .and_then(|t| t.with_second(0))
                    .unwrap();

                let local_time_str =
                    match timezone_service.format_from_utc_with_timezone(utc_time, user_timezone) {
                        Ok(local_time) => {
                            // extract only time (HH:MM) from "YYYY-MM-DD HH:MM" format
                            if let Some(time_part) = local_time.split_whitespace().nth(1) {
                                time_part.to_string()
                            } else {
                                format!("{:02}:{:02}", hour, minute)
                            }
                        }
                        Err(_) => format!("{:02}:{:02}", hour, minute),
                    };

                format!("Every {} at {}", days_str, local_time_str)
            }
            Some(Recurrence::EveryXDays {
                interval,
                hour,
                minute,
            }) => {
                let utc_time = Utc::now()
                    .with_hour(*hour as u32)
                    .and_then(|t| t.with_minute(*minute as u32))
                    .and_then(|t| t.with_second(0))
                    .unwrap();

                let local_time_str =
                    match timezone_service.format_from_utc_with_timezone(utc_time, user_timezone) {
                        Ok(local_time) => {
                            if let Some(time_part) = local_time.split_whitespace().nth(1) {
                                time_part.to_string()
                            } else {
                                format!("{:02}:{:02}", hour, minute)
                            }
                        }
                        Err(_) => format!("{:02}:{:02}", hour, minute),
                    };

                format!("Every {} days at {}", interval, local_time_str)
            }
            None => "Not recurring".to_string(),
        }
    }

    /// Get user tasks to display in /list_tasks
    pub async fn get_user_tasks_embed(
        &self,
        user_id: u64,
        timezone_service: Arc<TimezoneService>,
    ) -> CreateEmbed {
        let tasks = self.get_user_tasks(user_id).await;

        if tasks.is_empty() {
            return CreateEmbed::default()
                .title("📝 Your Tasks")
                .description("You don't have any tasks yet!")
                .color(Color::DARK_GREY)
                .footer(CreateEmbedFooter::new(
                    "Use /add_task to create your first task",
                ));
        }

        // verify user's timezone
        let user_timezone = match timezone_service.get_user_timezone(user_id).await {
            Ok(Some(tz)) => tz,
            Ok(None) => {
                return CreateEmbed::default()
                .title("❌ Timezone Required")
                .description("Please set your timezone first using `/timezone` to see task times correctly")
                .color(Color::RED)
                .footer(CreateEmbedFooter::new(
                    "This ensures all times are displayed in your local timezone",
                ));
            }
            Err(_) => "UTC".to_string(),
        };

        // separate tasks
        let (single_tasks, recurrent_tasks) = self.separate_tasks_by_type(&tasks);
        let total_tasks = single_tasks.len() + recurrent_tasks.len();

        let mut embed = CreateEmbed::default()
            .title("📝 Your Tasks")
            .description(format!(
                "\n• You have {} task{} in total",
                total_tasks,
                if total_tasks != 1 { "s" } else { "" }
            ))
            .color(Color::BLUE);

        embed = embed.field("\n", "\n", false);

        // Single tasks
        if !single_tasks.is_empty() {
            let mut single_tasks_field = String::new();

            for task in &single_tasks {
                let scheduled_str =
                    task.scheduled_time
                        .map_or(
                            "⏰ Not scheduled".to_string(),
                            |utc_dt| match timezone_service
                                .format_from_utc_with_timezone(utc_dt, &user_timezone)
                            {
                                Ok(local_time) => format!("> {}", local_time),
                                Err(_) => format!("> {} (UTC)", utc_dt.format("%Y-%m-%d %H:%M")),
                            },
                        );

                single_tasks_field.push_str(&format!("#{} - __**{}**__\n\n", task.id, task.title));

                match &task.description {
                    Some(desc) if !desc.trim().is_empty() => {
                        single_tasks_field.push_str(&format!("   *{}*\n", desc));
                    }
                    _ => {
                        single_tasks_field.push_str("   *(no description)*\n");
                    }
                }

                single_tasks_field.push_str(&format!("   {}\n\n", scheduled_str));
            }

            embed = embed.field(
                format!(
                    "▬▬▬▬▬ **Single Tasks ({})** ▬▬▬▬▬\n\u{200B}",
                    single_tasks.len()
                ),
                single_tasks_field,
                false,
            );
        }

        // Weekly tasks
        if !recurrent_tasks.is_empty() {
            let mut recurrent_tasks_field = String::new();

            for task in &recurrent_tasks {
                let recurrence_str = self.format_recurrence_for_display_with_timezone(
                    &task.recurrence,
                    &timezone_service,
                    &user_timezone,
                );

                recurrent_tasks_field
                    .push_str(&format!("#{} - __**{}**__\n\n", task.id, task.title));

                match &task.description {
                    Some(desc) if !desc.trim().is_empty() => {
                        recurrent_tasks_field.push_str(&format!("   *{}*\n", desc));
                    }
                    _ => {
                        recurrent_tasks_field.push_str("   *(no description)*\n");
                    }
                }

                recurrent_tasks_field.push_str(&format!("   > {}\n\n", recurrence_str));
            }

            embed = embed.field("\n", "", false).field(
                format!(
                    "▬▬▬▬▬ **Weekly Tasks ({})** ▬▬▬▬▬\n\u{200B}",
                    recurrent_tasks.len()
                ),
                recurrent_tasks_field,
                false,
            );
        }

        // Footer
        let now = Utc::now();
        let local_time_str =
            match timezone_service.format_from_utc_with_timezone(now, &user_timezone) {
                Ok(local_time) => local_time,
                Err(_) => now.format("%Y-%m-%d %H:%M").to_string() + " (UTC)",
            };

        embed = embed.footer(CreateEmbedFooter::new(format!(
            "\n\u{200B}\nTimezone: {} - {}",
            user_timezone, local_time_str
        )));

        embed
    }

    /// Separate tasks by type (Single or Weekly)
    fn separate_tasks_by_type<'a>(&self, tasks: &'a [Task]) -> (Vec<&'a Task>, Vec<&'a Task>) {
        let mut single_tasks: Vec<&'a Task> =
            tasks.iter().filter(|t| t.recurrence.is_none()).collect();

        let mut recurrent_tasks: Vec<&'a Task> =
            tasks.iter().filter(|t| t.recurrence.is_some()).collect();

        single_tasks.sort_by_key(|t| t.id);
        recurrent_tasks.sort_by_key(|t| t.id);

        (single_tasks, recurrent_tasks)
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
        new_title: Option<String>,
        new_description: Option<String>,
        new_datetime_input: Option<String>,
        is_weekly_task: bool,
        timezone_service: Arc<TimezoneService>,
    ) -> Result<Task, String> {
        // validate task exists and belongs to user
        let current_task = self
            .get_task_for_editing(task_id, user_id)
            .await
            .ok_or_else(|| "Task not found or you don't have permission to edit it".to_string())?;

        let (new_scheduled_time, new_recurrence) = if let Some(datetime_input) = new_datetime_input
        {
            let task_type = if is_weekly_task { "weekly" } else { "single" };
            let (scheduled_time, recurrence) = timezone_service
                .parse_task_input(&datetime_input, task_type, user_id)
                .await?;

            // calculate first ocurrence for weekly tasks
            if is_weekly_task {
                if let Some(Recurrence::Weekly { days, hour, minute }) = recurrence {
                    let first_time = self
                        .calculate_first_occurrence(&days, hour, minute)
                        .ok_or("Could not calculate first occurrence".to_string())?;

                    (
                        Some(first_time),
                        Some(Recurrence::Weekly { days, hour, minute }),
                    )
                } else {
                    return Err("Invalid recurrence type".to_string());
                }
            } else {
                (scheduled_time, recurrence)
            }
        } else {
            (current_task.scheduled_time, current_task.recurrence)
        };

        // validates title is not empty if a new one is provided
        if let Some(ref title) = new_title {
            if title.trim().is_empty() {
                return Err("Task title cannot be empty".to_string());
            }
        }

        // handle description
        let final_description = match new_description {
            Some(desc) if desc.trim().is_empty() => Some("".to_string()), // clear description
            Some(desc) => Some(desc), // update with new description
            None => None,             // don't change existing description
        };

        self.task_repo.edit_task(
            task_id,
            new_title,
            final_description,
            new_scheduled_time,
            new_recurrence,
            None,
        )
    }

    // === SCHEDULER BUSINESS LOGIC ===

    /// Get all tasks for scheduling (no user filtering)
    pub async fn get_all_tasks_for_scheduling(&self) -> Vec<Task> {
        self.task_repo.list_tasks()
    }

    /// Get task by ID (for scheduler and orchestrator use)
    pub async fn get_task_by_id(&self, task_id: u64) -> Option<Task> {
        self.task_repo
            .list_tasks()
            .into_iter()
            .find(|task| task.id == task_id)
    }
}
