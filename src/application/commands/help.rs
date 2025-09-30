use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::prelude::Context;
use serenity::builder::CreateInteractionResponse;

// Register /help command
pub fn register_help_command() -> CreateCommand {
    CreateCommand::new("help").description("Show available commands")
}

// Executes /help command
pub async fn run_help_command(ctx: &Context, command: &CommandInteraction) {
    if let Err(err) = command.create_response(&ctx.http, CreateInteractionResponse::Message(
        serenity::builder::CreateInteractionResponseMessage::default()
            .content(
                "Available commands:\n\
                /help - Show this message\n\
                /ping - Connection testing\n\
                /reminder - Create a reminder"
            )
    )).await
    {
        eprintln!("Error when answering /help: {:?}", err);
    }
}
