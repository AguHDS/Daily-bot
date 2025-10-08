use crate::application::services::config_service::ConfigService;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CreateInteractionResponse},
    builder::{CreateCommand, CreateInteractionResponseMessage},
    prelude::*,
};
use std::sync::Arc;

/// Register the /set_notification_channel command
pub fn register_set_notification_channel_command() -> CreateCommand {
    CreateCommand::new("set_notification_channel")
        .description("Set the channel where task notifications will be sent")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                serenity::model::prelude::CommandOptionType::Channel,
                "channel",
                "Select the channel for notifications",
            )
            .required(true),
        )
}

/// Run the command to set the notification channel
pub async fn run_set_notification_channel(
    ctx: &Context,
    command: &CommandInteraction,
    config_service: &Arc<ConfigService>,
) {
    let guild_id = match config_service
        .validate_guild_context(command.guild_id.map(|gid| gid.get()))
        .await
    {
        Ok(gid) => gid,
        Err(error) => {
            let builder =
                CreateInteractionResponseMessage::default().content(format!("❌ {}", error));
            let _ = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
                .await;
            return;
        }
    };

    // extract channel ID from command option
    let channel_id = match command
        .data
        .options
        .get(0)
        .and_then(|opt| match &opt.value {
            CommandDataOptionValue::Channel(channel_id) => Some(*channel_id),
            _ => None,
        }) {
        Some(c) => c,
        None => {
            let builder = CreateInteractionResponseMessage::default()
                .content("❌ Please provide a valid channel.");
            let _ = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
                .await;
            return;
        }
    };

    // delegate to ConfigService for business logic
    match config_service
        .set_notification_channel(guild_id, channel_id.get())
        .await
    {
        Ok(()) => {
            let builder = CreateInteractionResponseMessage::default().content(format!(
                "✅ Notifications will now be sent in <#{}>",
                channel_id
            ));
            let _ = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
                .await;
        }
        Err(error) => {
            let builder = CreateInteractionResponseMessage::default()
                .content(format!("❌ Failed to set notification channel: {}", error));
            let _ = command
                .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
                .await;
        }
    }
}
