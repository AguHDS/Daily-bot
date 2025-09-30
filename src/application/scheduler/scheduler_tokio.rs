use std::sync::Arc;
use tokio::time::{sleep, Duration};
use chrono::Utc;
use serenity::prelude::Context;
use crate::application::repositories::task_repository::TaskRepository;

// Scheduler loop that periodically checks for tasks and triggers reminders when scheduled time is reached
pub async fn start_scheduler(ctx: Arc<Context>, repo: Arc<TaskRepository>) {
    tokio::spawn(async move {
        loop {
            let now = Utc::now();
            let tasks = repo.list_tasks();

            for task in tasks {
                if task.scheduled_time <= now && !task.completed {
                    // for now, just print the reminder, later will be sent it to discord)
                    println!(
                        "[SCHEDULER] Reminder for user {}: {}",
                        task.user_id, task.message
                    );

                    // mark task as completed so it doesn't get triggered again immediately
                    repo.complete_task(task.id);
                }
            }

            // check every 60 seconds (can be tuned)
            sleep(Duration::from_secs(60)).await;
        }
    });
}
