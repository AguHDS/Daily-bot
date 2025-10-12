use crate::application::services::config_service::ConfigService;
use crate::domain::entities::task::{NotificationMethod, Task};
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::Context;
use std::sync::Arc;

#[derive(Clone)]
pub struct NotificationService;

impl NotificationService {
    pub fn new() -> Self {
        Self
    }

    /// Sends a notification for a task according to its NotificationMethod. If Channel/Both, uses the configured server notification channel
    pub async fn send_task_notification(
        &self,
        task: &Task,
        ctx: &Context,
        config_service: &ConfigService,
        guild_id: Option<u64>,
    ) -> Result<(), String> {
        match task.notification_method {
            NotificationMethod::DM => {
                self.send_dm(task, ctx).await?;
            }
            NotificationMethod::Channel => {
                self.send_channel_with_service(task, ctx, config_service, guild_id)
                    .await?;
            }
            NotificationMethod::Both => {
                self.send_dm(task, ctx).await?;
                self.send_channel_with_service(task, ctx, config_service, guild_id)
                    .await?;
            }
        }
        Ok(())
    }

    /// Send a direct message to the user
    async fn send_dm(&self, task: &Task, ctx: &Context) -> Result<(), String> {
        let user_id = UserId::from(task.user_id);

        user_id
            .create_dm_channel(&ctx.http)
            .await
            .map_err(|e| format!("Failed to create DM channel for user {}: {}", user_id, e))?
            .say(&ctx.http, &task.message)
            .await
            .map_err(|e| format!("Failed to send DM to user {}: {}", user_id, e))?;

        println!("📨 DM sent to user {} for task #{}", user_id, task.id);
        Ok(())
    }

    /// Send a message to the server's notification channel using ConfigService
    async fn send_channel_with_service(
        &self,
        task: &Task,
        ctx: &Context,
        config_service: &ConfigService,
        guild_id: Option<u64>,
    ) -> Result<(), String> {
        let gid = guild_id.ok_or_else(|| {
            format!(
                "Task {} has no guild_id. Cannot send channel notification.",
                task.id
            )
        })?;

        let channel_id = config_service
            .get_notification_channel(gid)
            .await
            .ok_or_else(|| {
                format!(
                    "No notification channel set for guild {}. Skipping channel notification.",
                    gid
                )
            })?;

        let channel = ChannelId::new(channel_id);
        channel
            .say(&ctx.http, &task.message)
            .await
            .map_err(|e| format!("Failed to send channel message for task {}: {}", task.id, e))?;

        println!(
            "📢 Channel notification sent for task #{} in guild {}",
            task.id, gid
        );
        Ok(())
    }

    /// Legacy method for compatibility (uses ConfigRepository directly)
    #[allow(dead_code)]
    async fn send_channel(
        &self,
        task: &Task,
        ctx: &Context,
        config_repo: &Arc<dyn crate::domain::repositories::ConfigRepository>,
        guild_id: Option<u64>,
    ) -> Result<(), String> {
        let gid = guild_id.ok_or_else(|| {
            format!(
                "Task {} has no guild_id. Cannot send channel notification.",
                task.id
            )
        })?;

        let channel_id = config_repo.get_notification_channel(gid).ok_or_else(|| {
            format!(
                "No notification channel set for guild {}. Skipping channel notification.",
                gid
            )
        })?;

        let channel = ChannelId::new(channel_id);
        channel
            .say(&ctx.http, &task.message)
            .await
            .map_err(|e| format!("Failed to send channel message for task {}: {}", task.id, e))?;

        println!(
            "Channel notification sent for task #{} in guild {}",
            task.id, gid
        );
        Ok(())
    }
}
