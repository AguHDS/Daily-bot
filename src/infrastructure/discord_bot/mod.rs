use crate::application::commands::{
    register_add_task_command, register_help_command, register_list_tasks_command,
    register_remove_task_command, run_add_task, run_help_command, run_list_tasks, run_remove_task,
};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;

use serenity::model::{
    application::Interaction,
    gateway::Ready,
    id::GuildId,
};
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_repo: Arc<TaskRepository>,
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        use serenity::builder::CreateCommand;

        // Register commands for each guild
        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;
            println!("Registering commands for guild: {}", guild_id.get());

            // Add Task
            let _ = guild_id
                .create_command(&ctx.http, register_add_task_command())
                .await;

            // List Tasks
            let _ = guild_id
                .create_command(&ctx.http, register_list_tasks_command())
                .await;

            // Remove Task
            let _ = guild_id
                .create_command(&ctx.http, register_remove_task_command())
                .await;

            // Help
            let _ = guild_id
                .create_command(&ctx.http, register_help_command())
                .await;

            println!("Commands registered for guild {}", guild_id.get());
        }

        // Start scheduler in the background
        start_scheduler(Arc::new(ctx), self.task_repo.clone());
        println!("Scheduler started");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "add_task" => run_add_task(&ctx, &command, &self.task_repo).await,
                "list_tasks" => run_list_tasks(&ctx, &command, &self.task_repo).await,
                "remove_task" => run_remove_task(&ctx, &command, &self.task_repo).await,
                "help" => run_help_command(&ctx, &command).await,
                _ => println!("Command not recognized: {}", command.data.name),
            }
        }
    }
}

// Run bot
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    // Task repository shared across handlers and scheduler
    let task_repo = Arc::new(TaskRepository::new());
    let handler = CommandHandler {
        task_repo: task_repo.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    println!("Starting client...");
    client.start().await?;
    Ok(())
}
