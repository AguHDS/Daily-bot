use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{
    ConfigRepository, TaskRepository, TaskSchedulerRepository, UserPreferencesRepository,
};
use crate::infrastructure::database::DatabaseManager;
use crate::infrastructure::repositories::{
    sqlite_config_repository::SqliteConfigRepository,
    sqlite_scheduler_repository::SqliteSchedulerRepository,
    sqlite_task_repository::SqliteTaskRepository,
    sqlite_user_preferences_repository::SqliteUserPreferencesRepository,
};
use crate::infrastructure::scheduler::priority_queue_scheduler::PriorityQueueScheduler;
use crate::infrastructure::timezone::timezone_manager::TimezoneManager;
use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{error, info};

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub task_orchestrator: Arc<TaskOrchestrator>,
    pub config_service: Arc<ConfigService>,
    pub notification_service: Arc<NotificationService>,
    pub timezone_service: Arc<TimezoneService>,
    pub sqlite_scheduler_repo: Arc<SqliteSchedulerRepository>,
}

impl CommandHandler {
    /// Register slash commands for a specific guild
    async fn register_commands_for_guild(&self, ctx: &Context, guild_id: GuildId) {
        // Test server ID
        const ALLOWED_GUILD_ID: u64 = 1422605167580155914;

        // Commands available for ALL guilds
        let mut commands = vec![
            crate::application::commands::register_add_task_command(),
            crate::application::commands::register_list_tasks_command(),
            crate::application::commands::register_remove_task_command(),
            crate::application::commands::register_help_command(),
            crate::application::commands::edit_task::register_edit_task_command(),
            crate::application::commands::set_notification_channel::register_set_notification_channel_command(),
            crate::application::commands::timezone::register_timezone_command(),
        ];

        // Only add stats command if it's the allowed server
        if guild_id.get() == ALLOWED_GUILD_ID {
            commands.push(crate::application::commands::register_stats_command());
        }

        if let Err(e) = guild_id.set_commands(&ctx.http, commands).await {
            error!("Failed to register commands for guild {}: {}", guild_id, e);
        }
    }
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Bot ready as {}", ready.user.name);

        for g in ready.guilds {
            self.register_commands_for_guild(&ctx, g.id).await;
        }

        // Load scheduled tasks on startup
        if let Err(e) = self
            .task_orchestrator
            .initialize_scheduler_with_existing_tasks()
            .await
        {
            error!("Failed to initialize scheduler: {}", e);
        }

        // Start priority queue worker loop
        PriorityQueueScheduler::start_scheduler(
            Arc::new(ctx),
            self.task_orchestrator.clone(),
            self.config_service.clone(),
            self.notification_service.clone(),
            self.sqlite_scheduler_repo.clone(),
        );

        info!("Scheduler started successfully");
    }

    async fn guild_create(
        &self,
        ctx: Context,
        guild: serenity::model::guild::Guild,
        is_new: Option<bool>,
    ) {
        if is_new.unwrap_or(false) {
            self.register_commands_for_guild(&ctx, guild.id).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "add_task" => {
                        crate::application::commands::add_task::run_add_task(
                            &ctx,
                            command,
                            &self.task_orchestrator,
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
                    "stats" => {
                        crate::application::commands::stats::run_stats(
                            &ctx,
                            command,
                            &self.task_service,
                        )
                        .await;
                    }
                    _ => {
                        crate::application::commands::interaction_handlers::handle_command(
                            &ctx,
                            &interaction,
                            &self.task_service,
                            &self.task_orchestrator,
                            &self.config_service,
                            &self.notification_service,
                            &self.timezone_service,
                        )
                        .await;
                    }
                }
            }
            Interaction::Component(_) => {
                crate::application::commands::interaction_handlers::handle_component(
                    &ctx,
                    &interaction,
                    &self.task_service,
                    &self.task_orchestrator,
                    &self.timezone_service,
                )
                .await;
            }
            Interaction::Modal(_) => {
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_orchestrator,
                    &self.timezone_service,
                    &self.config_service,
                )
                .await;
            }
            _ => {}
        }
    }
}

/// Composition root: builds all repos, services, and bot handler
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let db_path = "./data/bot.db";

    let db_manager = Arc::new(DatabaseManager::new(db_path)?);
    db_manager.initialize_database().await?;
    info!("Database initialized successfully");

    // SQLite repositories (all async now)
    let task_repo: Arc<dyn TaskRepository> = Arc::new(SqliteTaskRepository::new(db_path)?);
    let config_repo: Arc<dyn ConfigRepository> =
        Arc::new(SqliteConfigRepository::new(db_path).await?);
    let user_prefs_repo: Arc<dyn UserPreferencesRepository> =
        Arc::new(SqliteUserPreferencesRepository::new(db_path)?);

    // Persistent task scheduler repository
    let sqlite_scheduler_repo = Arc::new(SqliteSchedulerRepository::new(db_path)?);
    let task_scheduler: Arc<dyn TaskSchedulerRepository> = sqlite_scheduler_repo.clone();

    // Timezone manager
    let timezone_manager = Arc::new(
        TimezoneManager::new()
            .map_err(|e| format!("Failed to initialize timezone manager: {}", e))?,
    );

    // Services
    let notification_service = Arc::new(NotificationService::new());

    let config_service = Arc::new(ConfigService::new(config_repo.clone()));

    let timezone_service = Arc::new(TimezoneService::new(
        user_prefs_repo.clone(),
        timezone_manager,
    ));

    let task_service = Arc::new(TaskService::new(
        task_repo.clone(),
        config_repo.clone(),
        notification_service.clone(),
        timezone_service.clone(),
    ));

    let task_orchestrator = Arc::new(TaskOrchestrator::new(
        task_service.clone(),
        task_scheduler.clone(),
        timezone_service.clone(),
    ));

    let handler = CommandHandler {
        task_service,
        task_orchestrator,
        config_service,
        notification_service,
        timezone_service,
        sqlite_scheduler_repo: sqlite_scheduler_repo,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}
