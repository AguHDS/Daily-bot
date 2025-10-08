use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_service::TaskService;
use chrono::Utc;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Scheduler loop for reviewing tasks periodically and triggering notifications
pub fn start_scheduler(
    ctx: Arc<Context>,
    task_service: Arc<TaskService>,
    config_service: Arc<ConfigService>,
    notification_service: Arc<NotificationService>,
) {
    tokio::spawn(async move {
        loop {
            let now = Utc::now();

            // delegate to TaskService for business logic
            let tasks = task_service.get_all_tasks_for_scheduling().await;

            for task in &tasks {
                if let Some(scheduled_time) = task.scheduled_time {
                    if scheduled_time <= now {
                        let guild_id = task.guild_id;

                        // ✅ DELEGATE to NotificationService for business logic
                        if let Err(err) = notification_service
                            .send_task_notification(task, &ctx, &config_service, Some(guild_id))
                            .await
                        {
                            eprintln!("Failed to send notification for task {}: {}", task.id, err);
                        }

                        // delegate to TaskService for business logic
                        // handle post-notification logic (remove single tasks / reschedule recurring tasks)
                        if let Err(err) = task_service.handle_post_notification_task(&task).await {
                            eprintln!(
                                "Failed to handle post-notification for task {}: {}",
                                task.id, err
                            );
                        }
                    }
                }
            }

            // Check every minute
            sleep(Duration::from_secs(60)).await;
        }
    });
}
