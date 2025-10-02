use crate::application::repositories::task_repository::TaskRepository;
use chrono::Utc;
use serenity::model::id::UserId;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Scheduler loop for review tasks periodically and triggers reminders when appropriate
pub fn start_scheduler(ctx: Arc<Context>, repo: Arc<TaskRepository>) {
    tokio::spawn(async move {
        println!("[SCHEDULER] Scheduler started");

        loop {
            let now = Utc::now();
            let tasks = repo.list_tasks();

            for task in tasks {
                if task.scheduled_time <= Some(now) && !task.completed {
                    // convert u64 to userID
                    let user_id = UserId::from(task.user_id);

                    // obtain username from user ID
                    let user_name = match ctx.http.get_user(user_id).await {
                        Ok(user) => format!("{} ({})", user.name, user.id),
                        Err(_) => format!("Unknown ({})", task.user_id),
                    };

                    println!(
                        "[SCHEDULER] Reminder for user {}: {}",
                        user_name, task.message
                    );

                    // mark task as completed to avoid repeated reminders
                    repo.complete_task(task.id);
                }
            }

            sleep(Duration::from_secs(60)).await;
        }
    });
}
