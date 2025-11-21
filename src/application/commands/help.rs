use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::prelude::*;
use tracing::{error};

pub fn register_help_command() -> CreateCommand {
    CreateCommand::new("help").description("Show available commands")
}

pub async fn run_help_command(ctx: &Context, command: &CommandInteraction) {
    let content = "\
**Available Commands:**\n\
`/add_task` - Create a Single or Weekly task (Single tasks are removed after notification, Weekly task are automatically rescheduled) \n\
`/list_tasks` - List all your tasks\n\
`/remove_task` - Remove specific tasks or all of them\n\
`/edit_task` - Edit a task by selecting it\n\
`/timezone` - Set your current timezone based on your country, city or state\n\
`/set_notification_channel` - Set the channel where the bot will send notifications (admin only)\n\
`/help` - Show this message";

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default()
            .content(content)
            .ephemeral(false),
    );

    if let Err(err) = command.create_response(&ctx.http, builder).await {
        error!("Error executing /help: {:?}", err);
    }
}
