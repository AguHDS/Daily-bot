use crate::application::repositories::task_repository::TaskRepository;
use chrono::Utc;
use serenity::model::id::UserId;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Scheduler loop for reviewing tasks periodically and triggering reminders
pub fn start_scheduler(ctx: Arc<Context>, repo: Arc<dyn TaskRepository>) {
    tokio::spawn(async move {

        loop {
            let now = Utc::now();
            let tasks = repo.list_tasks();

            for task in &tasks {
                if task.scheduled_time <= Some(now) {
                    // convert u64 to UserId
                    let user_id = UserId::from(task.user_id);

                    let user_name = match ctx.http.get_user(user_id).await {
                        Ok(user) => format!("{} ({})", user.name, user.id),
                        Err(_) => format!("Unknown ({})", task.user_id),
                    };

                    println!("Reminder for user {}: {}", user_name, task.message);

                    // delete single tasks after sending reminder
                    if task.recurrence.is_none() {
                        let removed = repo.remove_task(task.id);
                        if removed {
                            println!(
                                "Single task #{} for user {} removed after reminder",
                                task.id, user_name
                            );
                        }
                    }
                }
            }

            sleep(Duration::from_secs(60)).await;
        }
    });
}
