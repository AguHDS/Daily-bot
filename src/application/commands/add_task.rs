use crate::application::repositories::task_repository::TaskRepository;
use chrono::{DateTime, Utc};
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

// Registers /add_task command
pub fn register_add_task_command() -> CreateCommand {
    CreateCommand::new("add_task")
        .description("Add a new task")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "message",
                "Task description",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "repeat_daily",
                "Should the task repeat daily?",
            )
            .required(false),
        )
}

// Execute /add_task command logic
pub async fn run_add_task(ctx: &Context, command: &CommandInteraction, repo: &TaskRepository) {
    let options = &command.data.options;

    // extract required message
    let message = match options.get(0) {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::String(s) => s.clone(),
            _ => {
                let builder = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("Error: invalid message provided")
                        .ephemeral(true),
                );
                let _ = command.create_response(&ctx.http, builder).await;
                return;
            }
        },
        None => {
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("Error: invalid message provided")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, builder).await;
            return;
        }
    };

    // extract the optional repeat_daily flag (default to false)
    let repeat_daily = match options.get(1) {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::Boolean(b) => *b,
            _ => false,
        },
        None => false,
    };

    // default current time: now
    let scheduled_time: DateTime<Utc> = Utc::now();

    // create the task in repo
    let task_id = repo.add_task(
        command.user.id.get(),
        message.clone(),
        scheduled_time,
        repeat_daily,
    );

    // answer to user
    let response_content = format!("Task created with ID **{}**: {}", task_id, message);
    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default().content(response_content),
    );

    let _ = command.create_response(&ctx.http, builder).await;
}
