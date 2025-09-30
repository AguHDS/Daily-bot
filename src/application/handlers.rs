use crate::application::commands::help::{run_help_command, register_help_command};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;
use serenity::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler;

#[async_trait]
impl EventHandler for CommandHandler {
    // Executed every time someone uses a slash command
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        use serenity::model::application::Interaction;

        if let Interaction::Command(ref command) = interaction {
            match command.data.name.as_str() {
                "help" => run_help_command(&ctx, command).await,
                _ => println!("Command not registered: {}", command.data.name),
            }
        }
    }

    // Executed when the bot connects, register commands and start scheduler
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        use serenity::model::id::GuildId;

        // Server ID
        let guild_id = GuildId::new(1422605167580155914);

        // Register commands
        let commands: Vec<CreateCommand> = vec![register_help_command()];
        if let Err(err) = guild_id.set_commands(&ctx.http, commands).await {
            eprintln!("Error trying to register commands: {}", err);
        }
        println!("Commands registered");

        // Initialize task repository (shared with scheduler)
        let task_repo = Arc::new(TaskRepository::new());

        // Start scheduler
        start_scheduler(Arc::new(ctx), task_repo.clone()).await;
    }
}
