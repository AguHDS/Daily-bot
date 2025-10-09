use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

pub fn register_list_tasks_command() -> CreateCommand {
    CreateCommand::new("list_tasks").description("📋 Show your current tasks")
}

pub async fn run_list_tasks(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>, // 🆕 Nuevo parámetro
) {
    let user_id: u64 = command.user.id.into();

    // pass timezone_service to format method
    let content = task_service
        .get_user_tasks_formatted(user_id, timezone_service.clone())
        .await;

    // send response
    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default()
            .content(content)
            .ephemeral(true),
    );

    if let Err(e) = command.create_response(&ctx.http, builder).await {
        eprintln!("Failed to send list_tasks response: {}", e);
    }
}
