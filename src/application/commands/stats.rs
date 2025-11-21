use crate::application::services::task_service::TaskService;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{error};

pub fn register_stats_command() -> CreateCommand {
    CreateCommand::new("stats")
        .description("📊 Show bot statistics (creator only)")
        .dm_permission(false) // No permitir en DMs
        .default_member_permissions(Permissions::empty()) // Sin permisos especiales por defecto
}

pub async fn run_stats(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
) {
    // Verify that the user is the bot creator in the test server
    if !is_authorized_user(&command.user) {
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default()
                .content("❌ You are not authorized to use this command.")
                .ephemeral(true),
        );
        
        if let Err(e) = command.create_response(&ctx.http, builder).await {
            error!("Failed to send stats unauthorized response: {}", e);
        }
        return;
    }

    // Get statistics
    let total_tasks = match task_service.get_total_task_count().await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to get total tasks count: {}", e);
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Failed to retrieve statistics.")
                    .ephemeral(true),
            );
            if let Err(e) = command.create_response(&ctx.http, builder).await {
                error!("Failed to send stats error response: {}", e);
            }
            return;
        }
    };

    // Get the number of servers using Serenity's cache
    let server_count = ctx.cache.guilds().len();

    // Create response embed
    let embed = CreateInteractionResponseMessage::default()
        .embed(
            serenity::builder::CreateEmbed::default()
                .title("Statistics")
                .description("Private statistics")
                .field("All tasks", format!("{}", total_tasks), true)
                .field("Servers registered", format!("{}", server_count), true)
                .color(0x00FF00) // Green
        )
        .ephemeral(true); // Visible only to the user who executed the command

    let builder = CreateInteractionResponse::Message(embed);

    if let Err(e) = command.create_response(&ctx.http, builder).await {
        error!("Failed to send stats response: {}", e);
    }
}

/// Verifies if the user is authorized (only the bot creator)
fn is_authorized_user(user: &User) -> bool {
    const CREATOR_ID: u64 = 300869447475003393;
    
    user.id == CREATOR_ID
}