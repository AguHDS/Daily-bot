use crate::domain::entities::scheduled_task::ScheduledTask;
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

    /// Sends a notification for a task according to its NotificationMethod.
    /// For Channel/Both, uses the task-specific channel_id
    pub async fn send_task_notification(&self, task: &Task, ctx: &Context) -> Result<(), String> {
        match task.notification_method {
            NotificationMethod::DM => {
                self.send_dm(task, ctx).await?;
            }
            NotificationMethod::Channel => {
                self.send_channel_with_task_channel(task, ctx).await?;
            }
            NotificationMethod::Both => {
                self.send_dm(task, ctx).await?;
                self.send_channel_with_task_channel(task, ctx).await?;
            }
        }
        Ok(())
    }

    /// Sends notification for a scheduled task (used by priority queue scheduler)
    pub async fn send_task_notification_from_scheduled(
        &self,
        scheduled_task: &ScheduledTask,
        ctx: &Context,
        task_orchestrator: &crate::application::services::task_orchestrator::TaskOrchestrator,
    ) -> Result<(), String> {
        // Fetch the full task details including description and channel_id
        let full_task = task_orchestrator
            .get_task_by_id(scheduled_task.task_id)
            .await;

        // Create task struct for notification - use full task if available, otherwise fallback to scheduled task data
        let notification_task = if let Some(task) = full_task {
            task
        } else {
            // Fallback if full task is not found (shouldn't normally happen)
            Task {
                id: scheduled_task.task_id,
                user_id: scheduled_task.user_id,
                guild_id: scheduled_task.guild_id,
                title: scheduled_task.title.clone(),
                description: None,
                scheduled_time: Some(scheduled_task.scheduled_time),
                recurrence: None,
                notification_method: scheduled_task.notification_method.clone(),
                channel_id: None, // No channel_id in fallback
                mention: scheduled_task.mention.clone(),
            }
        };

        // Send notification using task-specific channel
        self.send_task_notification(&notification_task, ctx).await
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

    /// Send a message to the task-specific channel with an embed
    pub async fn send_channel_with_task_channel(
        &self,
        task: &Task,
        ctx: &Context,
    ) -> Result<(), String> {
        let channel_id = task.channel_id.ok_or_else(|| {
            format!(
                "Task {} has no channel_id configured for channel notification.",
                task.id
            )
        })?;

        let channel = ChannelId::new(channel_id);

        // Create notification message with mention based on task configuration
        let notification_content = if let Some(mention) = &task.mention {
            // Use the specified mention(s) instead of the task creator
            format!("Your task is ready! {}", mention)
        } else {
            // Fallback to mentioning the task creator
            let user_mention = format!("<@{}>", task.user_id);
            format!("Your task is ready! {}", user_mention)
        };

        let embed = self.create_task_embed(task);
        let msg = CreateMessage::new()
            .content(notification_content)
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

            embed = embed.field("\u{2800}", format!("> {}", formatted), false);
        }

        embed
    }
}
