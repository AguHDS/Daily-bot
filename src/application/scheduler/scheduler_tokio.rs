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
                if let Some(scheduled_time) = task.scheduled_time {
                    if scheduled_time <= now {
                        // convert u64 to UserId
                        let user_id = UserId::from(task.user_id);

                        let user_name = match ctx.http.get_user(user_id).await {
                            Ok(user) => format!("{} ({})", user.name, user.id),
                            Err(_) => format!("Unknown ({})", task.user_id),
                        };

                        println!("Reminder for user {}: {}", user_name, task.message);

                        // if it's a single task, remove it
                        if task.recurrence.is_none() {
                            let removed = repo.remove_task(task.id);
                            if removed {
                                println!(
                                    "✅ Single task #{} for user {} removed after reminder",
                                    task.id, user_name
                                );
                            }
                        } else {
                            // if it's weekly(recurring), calculate next occurrence and update
                            if let Some(next_time) = task.next_occurrence() {
                                match repo.update_task_time(task.id, next_time) {
                                    Ok(_) => {
                                        println!(
                                            "♻️ Recurring task #{} for user {} rescheduled for {}",
                                            task.id, user_name, next_time
                                        );
                                    }
                                    Err(err) => eprintln!(
                                        "❌ Failed to update recurring task #{}: {}",
                                        task.id, err
                                    ),
                                }
                            } else {
                                eprintln!(
                                    "⚠️ Could not determine next occurrence for recurring task #{}",
                                    task.id
                                );
                            }
                        }
                    }
                }
            }

            // check every minute
            sleep(Duration::from_secs(60)).await;
        }
    });
}
