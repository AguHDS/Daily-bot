use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{
    ConfigRepository, TaskRepository, TaskSchedulerRepository, UserPreferencesRepository,
};
use crate::features::server_specific::config::ServerConfig;
use crate::features::server_specific::config::nickname_config::NicknameConfig;
use crate::features::server_specific::{Feature, NicknameChangerService, NicknameScheduler};
use crate::infrastructure::database::DatabaseManager;
use crate::infrastructure::repositories::{
    sqlite_config_repository::SqliteConfigRepository,
    sqlite_scheduler_repository::SqliteSchedulerRepository,
    sqlite_task_repository::SqliteTaskRepository,
    sqlite_user_preferences_repository::SqliteUserPreferencesRepository,
};
use crate::infrastructure::scheduler::priority_queue_scheduler::PriorityQueueScheduler;
use crate::infrastructure::timezone::timezone_manager::TimezoneManager;
use serenity::http::Http;
use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info};

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub task_orchestrator: Arc<TaskOrchestrator>,
    pub config_service: Arc<ConfigService>,
    pub notification_service: Arc<NotificationService>,
    pub timezone_service: Arc<TimezoneService>,
    pub sqlite_scheduler_repo: Arc<SqliteSchedulerRepository>,
    pub nickname_changer_service: Option<Arc<NicknameChangerService>>,
}

impl CommandHandler {
    /// Register slash commands for a specific servers
    async fn register_commands_for_guild(&self, ctx: &Context, guild_id: GuildId) {
        // Test server ID
        const SERVER_FOR_STATS: u64 = 1422605167580155914;

        // Commands available for ALL servers
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
        if guild_id.get() == SERVER_FOR_STATS {
            commands.push(crate::application::commands::register_stats_command());
            info!("Registered stats command for guild: {}", guild_id);
        }

        if let Err(e) = guild_id.set_commands(&ctx.http, commands).await {
            error!("Failed to register commands for guild {}: {}", guild_id, e);
        }
    }

    /// Initialize joke features for specific server
    async fn initialize_joke_features(&self, guild_id: GuildId) {
        let my_server_id = 1422605167580155914; // My server

        if guild_id.get() != my_server_id {
            return;
        }

        info!("Initializing joke features for guild {}", guild_id);

        // Start nickname scheduler if the service exists
        if let Some(nickname_service) = &self.nickname_changer_service {
            let scheduler = NicknameScheduler::new(nickname_service.clone());
            scheduler.start().await;
            info!("Nickname scheduler started for guild {}", guild_id);
        }
    }
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Bot ready as {}", ready.user.name);

        for g in ready.guilds {
            self.register_commands_for_guild(&ctx, g.id).await;
            self.initialize_joke_features(g.id).await;
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
            info!("Bot joined new guild: {} ({})", guild.name, guild.id);
            self.register_commands_for_guild(&ctx, guild.id).await;
            self.initialize_joke_features(guild.id).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                info!(
                    "Received command: {} from user: {}",
                    command.data.name, command.user.id
                );
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
                debug!("Received component interaction");
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
                debug!("Received modal interaction");
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_orchestrator,
                    &self.timezone_service,
                    &self.config_service,
                )
                .await;
            }
            _ => {
                debug!("Received unknown interaction type");
            }
        }
    }
}

/// Composition root: builds all repos, services, and bot handler
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting bot initialization...");

    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let db_path = "./data/bot.db";

    let db_manager = Arc::new(DatabaseManager::new(db_path)?);
    db_manager.initialize_database().await?;
    info!("Database initialized successfully");

    // SQLite repositories (all sync)
    let task_repo: Arc<dyn TaskRepository> = Arc::new(SqliteTaskRepository::new(db_path)?);
    let config_repo: Arc<dyn ConfigRepository> =
        Arc::new(SqliteConfigRepository::new(db_path).await?);
    let user_prefs_repo: Arc<dyn UserPreferencesRepository> =
        Arc::new(SqliteUserPreferencesRepository::new(db_path)?);

    // Persistent task scheduler repository
    let sqlite_scheduler_repo = Arc::new(SqliteSchedulerRepository::new(db_path)?);
    let task_scheduler: Arc<dyn TaskSchedulerRepository> = sqlite_scheduler_repo.clone();

    let timezone_manager = Arc::new(
        TimezoneManager::new()
            .map_err(|e| format!("Failed to initialize timezone manager: {}", e))?,
    );

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

    let nickname_changer_service = initialize_joke_services(&token).await;

    let handler = CommandHandler {
        task_service,
        task_orchestrator,
        config_service,
        notification_service,
        timezone_service,
        sqlite_scheduler_repo,
        nickname_changer_service,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}

// ==================== Additional features for my server ====================

async fn initialize_joke_services(token: &str) -> Option<Arc<NicknameChangerService>> {
    let server_config = ServerConfig::my_server();

    // Check if features are enabled
    if !server_config
        .enabled_features
        .contains(&Feature::NicknameChanger)
    {
        info!(
            "No joke features enabled for server {}",
            server_config.server_id
        );
        return None;
    }

    // Cargar configuración desde JSON
    let nickname_config = match NicknameConfig::load() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load nickname config: {}", e);
            return None;
        }
    };

    // Cargar pool de nicknames desde JSON
    let nicknames_pool = match NicknameConfig::load_nicknames() {
        Ok(nicknames) => nicknames,
        Err(e) => {
            error!("Failed to load nicknames pool: {}", e);
            return None;
        }
    };

    if nicknames_pool.is_empty() {
        error!("No nicknames available in the pool");
        return None;
    }

    info!(
        "Loaded {} nicknames and {} targets",
        nicknames_pool.len(),
        nickname_config.targets.len()
    );

    Some(Arc::new(NicknameChangerService::new(
        server_config,
        nickname_config,
        nicknames_pool,
        Arc::new(Http::new(token)),
    )))
}
