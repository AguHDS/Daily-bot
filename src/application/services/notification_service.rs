use crate::application::services::config_service::ConfigService;
use crate::domain::entities::task::{NotificationMethod, Task};
use chrono::Local;
use serenity::builder::{CreateEmbed, CreateMessage};
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

        let channel = ChannelId::new(channel_id);

        let user_mention = format!("<@{}>", task.user_id);

        let embed = self.create_task_embed(task);
        let msg = CreateMessage::new()
            .content(format!("Hey {}, your task is ready!", user_mention))
            .embed(embed);

        channel
            .send_message(&ctx.http, msg)
            .await
            .map_err(|e| format!("Failed to send channel message for task {}: {}", task.id, e))?;

        Ok(())
    }

    /// Create a rich embed for task notifications
    fn create_task_embed(&self, task: &Task) -> CreateEmbed {
        let task_type = if task.recurrence.is_some() {
            "Recurring"
        } else {
            "One-time"
        };

        let description = if let Some(desc) = &task.description {
            if !desc.trim().is_empty() {
                format!("{}", desc)
            } else {
                "_(no description)_".to_string()
            }
        } else {
            "_(no description)_".to_string()
        };

        let mut embed = CreateEmbed::new()
            .title(format!("{}", task.title))
            .color(Color::from_rgb(66, 135, 245))
            .description(description)
            .field("\u{2800}", "\u{200B}", false) // Espaciador
            .field("Task ID", format!("#{}", task.id), true)
            .field("Type", task_type, true);

        if let Some(scheduled_time) = task.scheduled_time {
            let local_time = scheduled_time.with_timezone(&Local);
            let formatted = local_time.format("%A, %d - %B - %Y at %H:%M").to_string();

            // Agregamos un field al final para la hora en gris
            embed = embed.field("\u{2800}", format!("> {}", formatted), false);
        }

        embed
    }
}
