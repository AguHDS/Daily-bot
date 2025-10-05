use crate::application::commands::{
    edit_task, interaction_handlers, register_add_task_command, register_help_command,
    register_list_tasks_command, register_remove_task_command,
};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;
use crate::infrastructure::repositories::json_task_repository::JsonTaskRepository;

use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_repo: Arc<dyn TaskRepository>,
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;

            let _ = guild_id
                .create_command(&ctx.http, register_add_task_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_list_tasks_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_remove_task_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_help_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, edit_task::register_edit_task_command())
                .await;
        }

        start_scheduler(Arc::new(ctx), self.task_repo.clone());
        println!("Scheduler started");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        println!("Received interaction: {:?}", interaction.kind());

        //delegate slash commands
        interaction_handlers::handle_command(&ctx, &interaction, &self.task_repo).await;

        //delegate component interactions
        interaction_handlers::handle_component(&ctx, &interaction, &self.task_repo).await;

        // dlegate modal submissions
        interaction_handlers::handle_modal(&ctx, &interaction, &self.task_repo).await;
    }
}

pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let task_repo: Arc<dyn TaskRepository> = Arc::new(JsonTaskRepository::new("tasks.json"));

    let handler = CommandHandler {
        task_repo: task_repo.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}
