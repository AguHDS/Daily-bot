use crate::application::services::config_service::ConfigService;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CreateInteractionResponse},
    builder::{CreateCommand, CreateInteractionResponseMessage, EditInteractionResponse},
    prelude::*,
};
use std::sync::Arc;

pub fn register_set_notification_channel_command() -> CreateCommand {
    CreateCommand::new("set_notification_channel")
        .description("Set the channel where task notifications will be sent (Admin only)")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                serenity::model::prelude::CommandOptionType::Channel,
                "channel",
                "Select the channel for notifications",
            )
            .required(true),
        )
}

pub async fn run_set_notification_channel(
    ctx: &Context,
    command: &CommandInteraction,
    config_service: &Arc<ConfigService>,
) {
    // Defer early to avoid Discord timeouts
    if let Err(_) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(false),
            ),
        )
        .await
    {
        return;
    }

    // Permission check
    let has_permission = command.member.as_ref().map_or(false, |member| {
        member
            .permissions
            .map_or(false, |perms| perms.administrator())
    });

    if !has_permission {
        let _ = command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("❌ You need **administrator** permissions to use this command"),
            )
            .await;
        return;
    }

    // Validate guild
    let guild_id = match config_service
        .validate_guild_context(command.guild_id.map(|gid| gid.get()))
        .await
    {
        Ok(gid) => gid,
        Err(error) => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!("❌ {}", error)),
                )
                .await;
            return;
        }
    };

    // Extract channel
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
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("❌ Select a valid channel"),
                )
                .await;
            return;
        }
    };

    // Call service
    match config_service
        .set_notification_channel(guild_id, channel_id.get())
        .await
    {
        Ok(()) => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "✅ Notifications will now be sent in <#{}>",
                        channel_id
                    )),
                )
                .await;
        }
        Err(error) => {
            let _ = command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content(format!("❌ Failed to set notification channel: {}", error)),
                )
                .await;
        }
    }
}
