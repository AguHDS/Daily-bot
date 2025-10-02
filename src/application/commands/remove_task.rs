use crate::application::repositories::task_repository::TaskRepository;
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

// Register /remove_task command
pub fn register_remove_task_command() -> CreateCommand {
    CreateCommand::new("remove_task")
        .description("Delete a task by ID")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "id", "Task ID")
                .required(true),
        )
}

// /remove_task logic
pub async fn run_remove_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_repo: &TaskRepository,
) {
    // Extract task ID
    let task_id = match command.data.options.get(0) {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::Integer(i) => *i as u64,
            _ => 0,
        },
        None => 0,
    };

    if task_repo.remove_task(task_id) {
        // remove_task already saves to JSON
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default()
                .content(format!("✅ Task {} removed", task_id)),
        );
        let _ = command.create_response(&ctx.http, builder).await;
    } else {
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default()
                .content("❌ Couldn't find task with the specified ID"),
        );
        let _ = command.create_response(&ctx.http, builder).await;
    }
}
