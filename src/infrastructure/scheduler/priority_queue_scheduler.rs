use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use chrono::Utc;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};
use tracing::{error};

/// Efficient scheduler using priority queue
pub struct PriorityQueueScheduler;

impl PriorityQueueScheduler {
    pub fn start_scheduler(
        ctx: Arc<Context>,
        task_orchestrator: Arc<TaskOrchestrator>,
        config_service: Arc<ConfigService>,
        notification_service: Arc<NotificationService>,
        scheduler_repo: Arc<crate::infrastructure::repositories::sqlite_scheduler_repository::SqliteSchedulerRepository>,
    ) {
        tokio::spawn(async move {
            // Subscribe to wake-up notifications
            let mut wakeup_receiver = scheduler_repo.subscribe_wakeup();
            
            loop {
                match Self::scheduler_iteration(
                    &ctx,
                    &task_orchestrator,
                    &config_service,
                    &notification_service,
                    &mut wakeup_receiver,
                )
                .await
                {
                    Ok(should_continue) => {
                        if !should_continue {
                            // No pending tasks, sleep for a while
                            tokio::select! {
                                _ = sleep(Duration::from_secs(300)) => {},
                                _ = wakeup_receiver.recv() => {}
                            }
                            continue;
                        }
                    }
                    Err(e) => {
                        error!("Scheduler iteration error: {}", e);
                        // wait 1m before retrying in case of error
                        sleep(Duration::from_secs(60)).await;
                    }
                }
            }
        });
    }

    async fn scheduler_iteration(
        ctx: &Context,
        task_orchestrator: &TaskOrchestrator,
        config_service: &ConfigService,
        notification_service: &NotificationService,
        wakeup_receiver: &mut broadcast::Receiver<()>,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now();

        // verify next scheduled task (pending)
        if let Some(next_task) = task_orchestrator.peek_next_scheduled_task().await? {
            if next_task.scheduled_time <= now {
                // task ready to notify
                
                Self::process_due_task(
                    ctx,
                    task_orchestrator,
                    config_service,
                    notification_service,
                    next_task,
                )
                .await?;
                
                return Ok(true); // continue immediatly (there might be more due tasks)
            } else {
                // Sleep until next task is due OR until interrupted by new task
                let time_until_task = (next_task.scheduled_time - now)
                    .to_std()
                    .unwrap_or(Duration::from_secs(1));
                
                // Use tokio::select! to sleep until task time OR wake-up signal
                tokio::select! {
                    _ = sleep(time_until_task) => {}
                    _ = wakeup_receiver.recv() => {}
                }
                
                return Ok(true);
            }
        } else {
            // no pending tasks
            return Ok(false);
        }
    }

    async fn process_due_task(
        ctx: &Context,
        task_orchestrator: &TaskOrchestrator,
        config_service: &ConfigService,
        notification_service: &NotificationService,
        scheduled_task: crate::domain::entities::scheduled_task::ScheduledTask,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // remove the task from the scheduler (it's already in scheduled_task)
        task_orchestrator.pop_next_scheduled_task().await?;

        // send notification
        if let Err(_err) = notification_service
            .send_task_notification_from_scheduled(&scheduled_task, ctx, config_service, task_orchestrator)
            .await
        {
            // reinsert task if notification failed (retry in 1 minute)
            let retry_time = Utc::now() + chrono::Duration::minutes(1);
            let mut retry_task = scheduled_task.clone();
            retry_task.scheduled_time = retry_time;
            task_orchestrator.add_scheduled_task(retry_task).await?;
            return Ok(());
        }

        // obtain repository's full response and handle post-notification via orchestrator
        if let Some(full_task) = task_orchestrator.get_task_by_id(scheduled_task.task_id).await {
            let _ = task_orchestrator.handle_post_notification_task(&full_task).await;
        }

        Ok(())
    }
}
