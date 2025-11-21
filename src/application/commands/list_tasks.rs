use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{error};

pub fn register_list_tasks_command() -> CreateCommand {
    CreateCommand::new("list_tasks").description("📋 Show your current tasks")
}

pub async fn run_list_tasks(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) {
    let user_id: u64 = command.user.id.into();
    let user_mention = command.user.mention();

    let embed_response = task_service
        .get_user_tasks_embed(user_id, timezone_service.clone())
        .await;

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default()
            .content(format!("{}, these are your tasks\n", user_mention))
            .add_embed(embed_response)
            .ephemeral(false),
    );

    if let Err(e) = command.create_response(&ctx.http, builder).await {
        error!("Failed to send list_tasks response: {}", e);
    }
}
