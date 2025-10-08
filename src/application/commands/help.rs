use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;

pub fn register_help_command() -> CreateCommand {
    CreateCommand::new("help").description("Show available commands")
}

pub async fn run_help_command(ctx: &Context, command: &CommandInteraction) {
    let content = "\
**Available Commands:**\n\
`/add_task` - Create a new task (single or weekly)\n\
`/list_tasks` - List all your tasks\n\
`/remove_task` - Remove specific tasks or all of them\n\
`/edit_task` - Edit a task by selecting it\n\
`/set_notification_channel` - Set the channel for task notifications\n\
`/help` - Show this help message";

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default()
            .content(content)
            .ephemeral(true),
    );

    if let Err(err) = command.create_response(&ctx.http, builder).await {
        eprintln!("Error executing /help: {:?}", err);
    }
}
