use crate::application::repositories::task_repository::TaskRepository;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::prelude::*;
use serenity::prelude::*;

// Registers /list_tasks command
pub fn register_list_tasks_command() -> CreateCommand {
    CreateCommand::new("list_tasks").description("Current task list")
}

// /list_tasks command logic
pub async fn run_list_tasks(
    ctx: &Context,
    command: &CommandInteraction,
    task_repo: &TaskRepository,
) {
    let tasks = task_repo.list_tasks();
    let user_id: u64 = command.user.id.into();
    let mut content = String::from("Your tasks:\n");

    for task in tasks.iter().filter(|t| t.user_id == user_id) {
        content.push_str(&format!(
            "{} - {} - {} - {}\n",
            task.id,
            task.message,
            task.scheduled_time
                .map_or("Not scheduled".to_string(), |dt| dt.to_string()),
            if task.completed { "✅" } else { "❌" }
        ));
    }

    if content == "Your tasks:\n" {
        content = "You don't have any tasks".to_string();
    }

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default().content(content),
    );

    // response to user
    let _ = command.create_response(&ctx.http, builder).await;
}
