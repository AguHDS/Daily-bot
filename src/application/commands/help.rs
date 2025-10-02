use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

// registers /help command
pub fn register_help_command() -> CreateCommand {
    CreateCommand::new("help").description("Show available commands")
}

// /help command logic
pub async fn run_help_command(ctx: &Context, command: &CommandInteraction) {
    let content = "\
Comandos disponibles:\n\
/help - Show this message\n\
/add_task - Create new task\n\
/list_tasks - Task list\n\
/remove_task - Delete a task by ID";

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default().content(content),
    );

    if let Err(err) = command.create_response(&ctx.http, builder).await {
        eprintln!("Error when /help: {:?}", err);
    }
}
