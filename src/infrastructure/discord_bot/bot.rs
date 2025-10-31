use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{
    ConfigRepository, TaskRepository, TaskSchedulerRepository, UserPreferencesRepository,
};
use crate::infrastructure::repositories::json_config_repository::JsonConfigRepository;
use crate::infrastructure::repositories::{
    json_task_repository::JsonTaskRepository,
    json_user_preferences_repository::JsonUserPreferencesRepository,
    memory_scheduler_repository::MemorySchedulerRepository,
};
use crate::infrastructure::scheduler::priority_queue_scheduler::PriorityQueueScheduler;
use crate::infrastructure::timezone::timezone_manager::TimezoneManager;
use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub task_orchestrator: Arc<TaskOrchestrator>,
    pub config_service: Arc<ConfigService>,
    pub notification_service: Arc<NotificationService>,
    pub timezone_service: Arc<TimezoneService>,
    pub memory_scheduler_repo: Arc<MemorySchedulerRepository>,
}

impl CommandHandler {
    /// Helper function to register commands for a specific guild
    async fn register_commands_for_guild(&self, ctx: &Context, guild_id: GuildId) {
        let commands = vec![
            crate::application::commands::register_add_task_command(),
            crate::application::commands::register_list_tasks_command(),
            crate::application::commands::register_remove_task_command(),
            crate::application::commands::register_help_command(),
            crate::application::commands::edit_task::register_edit_task_command(),
            crate::application::commands::set_notification_channel::register_set_notification_channel_command(),
            crate::application::commands::timezone::register_timezone_command(),
        ];

        match guild_id.set_commands(&ctx.http, commands).await {
            Ok(_) => println!("Commands registered for guild {}", guild_id),
            Err(e) => eprintln!(
                "Failed to register commands for guild {}: {}",
                guild_id, e
            ),
        }
    }
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        // register commands for existing guilds
        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;
            self.register_commands_for_guild(&ctx, guild_id).await;
        }

        // initialize scheduler with existing tasks
        if let Err(e) = self.task_orchestrator.initialize_scheduler_with_existing_tasks().await {
            eprintln!("Failed to initialize scheduler with existing tasks: {}", e);
        }

        // start priority queue
        PriorityQueueScheduler::start_scheduler(
            Arc::new(ctx),
            self.task_orchestrator.clone(),
            self.config_service.clone(),
            self.notification_service.clone(),
            self.memory_scheduler_repo.clone(),
        );

    }

    /// Handle when the bot joins a new guild
    async fn guild_create(
        &self,
        ctx: Context,
        guild: serenity::model::guild::Guild,
        is_new: Option<bool>,
    ) {
        println!("Bot joined new guild: {} ({})", guild.name, guild.id);

        // only register commands when is a new server (not one on cache)
        if is_new.unwrap_or(false) {
            println!("Registering commands for new guild...");
            self.register_commands_for_guild(&ctx, guild.id).await;
        } else {
            println!("Guild was cached, skipping command registration");
        }
    }

    /// Decide what to do depending on user's interaction type with the bot
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                "add_task" => {
                    crate::application::commands::add_task::run_add_task(
                        &ctx,
                        command,
                        &self.task_orchestrator, // ✅ CAMBIAR: Usar orchestrator en lugar de task_service
                        &self.timezone_service,
                    )
                    .await;
                }
                "set_notification_channel" => {
                    crate::application::commands::set_notification_channel::run_set_notification_channel(
                        &ctx,
                        command,
                        &self.config_service,
                    )
                    .await;
                }
                "timezone" => {
                    crate::application::commands::timezone::run_timezone_command(
                        &ctx,
                        command,
                        &self.timezone_service,
                    )
                    .await;
                }
                _ => {
                    crate::application::commands::interaction_handlers::handle_command(
                        &ctx,
                        &interaction,
                        &self.task_service, // Mantener task_service para consultas
                        &self.task_orchestrator, // Add orchestrator for remove operations
                        &self.config_service,
                        &self.notification_service,
                        &self.timezone_service,
                    )
                    .await;
                }
            },
            Interaction::Component(_) => {
                crate::application::commands::interaction_handlers::handle_component(
                    &ctx,
                    &interaction,
                    &self.task_service, // Mantener task_service para consultas
                    &self.task_orchestrator, // Add orchestrator for remove operations
                    &self.timezone_service,
                )
                .await;
            }
            Interaction::Modal(_) => {
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_orchestrator, // ✅ CAMBIAR: Usar orchestrator para creación/edición
                    &self.timezone_service,
                )
                .await;
            }
            _ => {}
        }
    }
}

/// Factory that initializes repositories, services, and the discord bot, and then boots it
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    // initialize repositories
    let task_repo: Arc<dyn TaskRepository> = Arc::new(JsonTaskRepository::new("./data/tasks.json"));
    let config_repo: Arc<dyn ConfigRepository> = Arc::new(JsonConfigRepository::new(
        "./data/channel_notification.json",
    ));
    let user_prefs_repo: Arc<dyn UserPreferencesRepository> = Arc::new(
        JsonUserPreferencesRepository::new("./data/user_timezone_config.json"),
    );
    // Create memory scheduler repository - need both concrete and trait references
    let memory_scheduler_repo = Arc::new(MemorySchedulerRepository::new());
    let task_scheduler: Arc<dyn TaskSchedulerRepository> = memory_scheduler_repo.clone();

    // initialize timezone manager
    let timezone_manager = Arc::new(
        TimezoneManager::new()
            .map_err(|e| format!("Failed to initialize timezone manager: {}", e))?,
    );

    // initialize services
    let notification_service = Arc::new(NotificationService::new());
    let config_service = Arc::new(ConfigService::new(config_repo.clone()));
    let timezone_service = Arc::new(TimezoneService::new(
        user_prefs_repo.clone(),
        timezone_manager.clone(),
    ));
    let task_service = Arc::new(TaskService::new(
        task_repo.clone(),
        config_repo.clone(),
        notification_service.clone(),
        timezone_service.clone(),
    ));

    // initialize task orchestrator
    let task_orchestrator = Arc::new(TaskOrchestrator::new(
        task_service.clone(),
        task_scheduler.clone(),
        timezone_service.clone(),
    ));

    let handler = CommandHandler {
        task_service: task_service.clone(),
        task_orchestrator: task_orchestrator.clone(),
        config_service: config_service.clone(),
        notification_service: notification_service.clone(),
        timezone_service: timezone_service.clone(),
        memory_scheduler_repo: memory_scheduler_repo.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}
