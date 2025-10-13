use crate::application::services::config_service::ConfigService;
use crate::domain::entities::task::{NotificationMethod, Task};
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::colour::Color;
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::Context;

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

    /// Send a direct message to the user with an embed
    pub async fn send_dm(&self, task: &Task, ctx: &Context) -> Result<(), String> {
        let user_id = UserId::from(task.user_id);

        let embed = self.create_task_embed(task);

        let dm_channel = user_id
            .create_dm_channel(&ctx.http)
            .await
            .map_err(|e| format!("Failed to create DM channel for user {}: {}", user_id, e))?;

        let msg = CreateMessage::new().embed(embed);

        dm_channel
            .send_message(&ctx.http, msg)
            .await
            .map_err(|e| format!("Failed to send DM to user {}: {}", user_id, e))?;

        println!("DM embed sent to user {} for task #{}", user_id, task.id);
        Ok(())
    }

    /// Send a message to the server's notification channel using ConfigService with an embed
    pub async fn send_channel_with_service(
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

        let embed = self.create_task_embed(task);
        let msg = CreateMessage::new().embed(embed);
        let channel = ChannelId::new(channel_id);

        channel
            .send_message(&ctx.http, msg)
            .await
            .map_err(|e| format!("Failed to send channel message for task {}: {}", task.id, e))?;

        println!(
            "Channel notification sent for task #{} in guild {}",
            task.id, gid
        );

        Ok(())
    }

    fn create_task_embed(&self, task: &Task) -> CreateEmbed {
        let mut embed = CreateEmbed::new()
            .title("⏰ Task Reminder")
            .color(Color::BLUE)
            .field("Task", &task.title, false);

        if let Some(description) = &task.description {
            if !description.trim().is_empty() {
                embed = embed.field("Description", description, false);
            }
        }

        // add task id and type information
        embed = embed.field("Task ID", format!("#{}", task.id), true);

        // add recurrence information
        let task_type = if task.recurrence.is_some() {
            "Recurring"
        } else {
            "One-time"
        };
        embed = embed.field("Type", task_type, true);

        // add scheduled time if available
        if let Some(scheduled_time) = task.scheduled_time {
            embed = embed.field(
                "Scheduled For",
                format!("<t:{}:F>", scheduled_time.timestamp()),
                false,
            );
        }

        // add notification method
        let method = match task.notification_method {
            NotificationMethod::DM => "Direct Message",
            NotificationMethod::Channel => "Channel",
            NotificationMethod::Both => "Both DM and Channel",
        };
        embed = embed.field("Notification", method, true);

        let footer = CreateEmbedFooter::new(format!("User ID: {}", task.user_id));
        embed = embed.footer(footer);

        embed
    }
}
