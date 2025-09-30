use crate::application::commands::help::{run_help_command, register_help_command};
use serenity::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub struct CommandHandler;

#[async_trait]
impl EventHandler for CommandHandler {
    // Executed every time someone uses slash command
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        use serenity::model::application::Interaction;

        if let Interaction::Command(ref command) = interaction {
            match command.data.name.as_str() {
                "help" => run_help_command(&ctx, command).await,
                _ => println!("Command not registered: {}", command.data.name),
            }
        }
    }

    // When the bot connects, register all commands
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        use serenity::model::id::GuildId;

        // server ID
        let guild_id = GuildId::new(1422605167580155914);

        let commands: Vec<CreateCommand> = vec![register_help_command()];

        if let Err(err) = guild_id.set_commands(&ctx.http, commands).await {
            eprintln!("Error trying to register commands: {}", err);
        }

        println!("Commands registered");
    }
}
