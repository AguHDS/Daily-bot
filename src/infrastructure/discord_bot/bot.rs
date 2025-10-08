use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_service::TaskService;
use crate::domain::repositories::{ConfigRepository, TaskRepository};
use crate::infrastructure::repositories::{
    config_repository::InMemoryConfigRepository, json_task_repository::JsonTaskRepository,
};
use crate::infrastructure::scheduler::scheduler_tokio::start_scheduler;
use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub config_service: Arc<ConfigService>,
    pub notification_service: Arc<NotificationService>,
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        // Register commands
        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;

            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::register_add_task_command(),
                )
                .await;
            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::register_list_tasks_command(),
                )
                .await;
            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::register_remove_task_command(),
                )
                .await;
            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::register_help_command(),
                )
                .await;
            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::edit_task::register_edit_task_command(),
                )
                .await;
            let _ = guild_id
                .create_command(
                    &ctx.http,
                    crate::application::commands::set_notification_channel::register_set_notification_channel_command(),
                )
                .await;
        }

        // Start scheduler with services
        start_scheduler(
            Arc::new(ctx),
            self.task_service.clone(),
            self.config_service.clone(),
            self.notification_service.clone(),
        );
        println!("Scheduler started");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        println!("Received interaction: {:?}", interaction.kind());

        match &interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                "set_notification_channel" => {
                    crate::application::commands::set_notification_channel::run_set_notification_channel(
                &ctx,
                command,
                &self.config_service,
            )
            .await;
                }
                _ => {
                    crate::application::commands::interaction_handlers::handle_command(
                        &ctx,
                        &interaction,
                        &self.task_service,
                        &self.config_service,
                        &self.notification_service,
                    )
                    .await;
                }
            },
            Interaction::Component(_) => {
                crate::application::commands::interaction_handlers::handle_component(
                    &ctx,
                    &interaction,
                    &self.task_service,
                )
                .await;
            }
            Interaction::Modal(_) => {
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_service,
                )
                .await;
            }
            _ => {}
        }
    }
}

pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    // Initialize repositories
    let task_repo: Arc<dyn TaskRepository> = Arc::new(JsonTaskRepository::new("tasks.json"));
    let config_repo: Arc<dyn ConfigRepository> = Arc::new(InMemoryConfigRepository::new());

    // Initialize services
    let notification_service = Arc::new(NotificationService::new());
    let config_service = Arc::new(ConfigService::new(config_repo.clone()));
    let task_service = Arc::new(TaskService::new(
        task_repo.clone(),
        config_repo.clone(),
        notification_service.clone(),
    ));

    let handler = CommandHandler {
        task_service: task_service.clone(),
        config_service: config_service.clone(),
        notification_service: notification_service.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    println!("Bot starting...");
    client.start().await?;
    Ok(())
}
