use crate::application::services::task_service::TaskService;
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
) {
    let user_id: u64 = command.user.id.into();

    // delegate to TaskService for business logic
    let content = task_service.get_user_tasks_formatted(user_id).await;

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
