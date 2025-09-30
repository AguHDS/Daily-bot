use crate::application::handlers::CommandHandler;
use serenity::prelude::*;

// Initialize and start bot
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");

    // intents = event types the bot will receive
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES;

    let mut client = Client::builder(&token, intents)
        .event_handler(CommandHandler)
        .await?;

    client.start().await?;
    Ok(())
}
