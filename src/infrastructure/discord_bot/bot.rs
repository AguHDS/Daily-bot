use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{
    TaskRepository, TaskSchedulerRepository, UserPreferencesRepository,
};
use crate::features::server_specific::{
    ServerFeaturesOrchestrator, ServerInteractionHandler, initialize_specific_services,
};
use crate::infrastructure::database::DatabaseManager;
use crate::infrastructure::repositories::{
    sqlite_scheduler_repository::SqliteSchedulerRepository,
    sqlite_task_repository::SqliteTaskRepository,
    sqlite_user_preferences_repository::SqliteUserPreferencesRepository,
};
use crate::infrastructure::scheduler::priority_queue_scheduler::PriorityQueueScheduler;
use crate::infrastructure::timezone::timezone_manager::TimezoneManager;
use crate::utils::ModalStorage;
use serenity::all::{GuildId, Interaction, Message, Ready, ResumedEvent};
use serenity::prelude::*;
use songbird::SerenityInit;
use std::sync::Arc;
use tracing::{debug, error, info};

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub task_orchestrator: Arc<TaskOrchestrator>,
    pub notification_service: Arc<NotificationService>,
    pub timezone_service: Arc<TimezoneService>,
    pub sqlite_scheduler_repo: Arc<SqliteSchedulerRepository>,
    pub server_features_orchestrator: Arc<ServerFeaturesOrchestrator>,
    pub server_interaction_handler: Arc<ServerInteractionHandler>,
    pub modal_storage: Arc<ModalStorage>,
}

impl CommandHandler {
    /// Register slash commands for a specific servers
    async fn register_commands_for_guild(&self, ctx: &Context, guild_id: GuildId) {
        // Test server ID
        const SERVER_FOR_STATS_COMMAND: u64 = 479788664876957737;

        // Commands available for ALL servers
        let mut commands = vec![
            crate::application::commands::register_add_task_command(),
            crate::application::commands::register_list_tasks_command(),
            crate::application::commands::register_remove_task_command(),
            crate::application::commands::register_help_command(),
            crate::application::commands::edit_task::register_edit_task_command(),
            crate::application::commands::timezone::register_timezone_command(),
        ];

        // Only add stats command if it's the allowed server
        if guild_id.get() == SERVER_FOR_STATS_COMMAND {
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
            self.server_features_orchestrator
                .initialize_server_features(g.id)
                .await;
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
            self.notification_service.clone(),
            self.sqlite_scheduler_repo.clone(),
        );

        info!("Scheduler started successfully");
    }

    async fn resume(&self, _ctx: Context, _resume: ResumedEvent) {
        info!("Bot reconnected to Discord gateway");
    }

    async fn guild_create(
        &self,
        ctx: Context,
        guild: serenity::model::guild::Guild,
        is_new: Option<bool>,
    ) {
        if is_new.unwrap_or(false) {
            self.register_commands_for_guild(&ctx, guild.id).await;
            self.server_features_orchestrator
                .initialize_server_features(guild.id)
                .await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                "add_task" => {
                    crate::application::commands::add_task::run_add_task(
                        &ctx,
                        command,
                        &self.task_orchestrator,
                        &self.timezone_service,
                        &self.modal_storage,
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
                        &self.notification_service,
                        &self.timezone_service,
                    )
                    .await;
                }
            },
            Interaction::Component(component) => {
                debug!(
                    "Received component interaction: {}",
                    component.data.custom_id
                );

                if component.data.custom_id.starts_with("kick_") {
                    self.server_interaction_handler
                        .handle_button_interaction(&ctx, component)
                        .await;
                } else {
                    crate::application::commands::interaction_handlers::handle_component(
                        &ctx,
                        &interaction,
                        &self.task_service,
                        &self.task_orchestrator,
                        &self.timezone_service,
                    )
                    .await;
                }
            }
            Interaction::Modal(_) => {
                debug!("Received modal interaction");
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_orchestrator,
                    &self.timezone_service,
                    &self.modal_storage,
                )
                .await;
            }
            _ => {
                debug!("Received unknown interaction type");
            }
        }
    }

    /// Handle message events for server-specific features
    async fn message(&self, ctx: Context, message: Message) {
        // Ignore messages from bots
        if message.author.bot {
            return;
        }

        // Handle server-specific message interactions
        self.server_interaction_handler
            .handle_message_interaction(&ctx, &message)
            .await;
    }
}

/// Composition root: builds all repos, services, and bot handler
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES;

    let db_path = "./data/bot.db";

    let db_manager = Arc::new(DatabaseManager::new(db_path)?);
    db_manager.initialize_database().await?;
    info!("Database initialized successfully");

    // SQLite repositories (all sync)
    let task_repo: Arc<dyn TaskRepository> = Arc::new(SqliteTaskRepository::new(db_path)?);
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

    let timezone_service = Arc::new(TimezoneService::new(
        user_prefs_repo.clone(),
        timezone_manager,
    ));

    let task_service = Arc::new(TaskService::new(
        task_repo.clone(),
        notification_service.clone(),
        timezone_service.clone(),
    ));

    let task_orchestrator = Arc::new(TaskOrchestrator::new(
        task_service.clone(),
        task_scheduler.clone(),
        timezone_service.clone(),
    ));

    let songbird = songbird::Songbird::serenity();

    // Initialize server-specific features
    let (nickname_changer_service, kick_service, voice_interaction_service) =
        initialize_specific_services(&token, songbird.clone()).await;

    // Initialize alias service
    let alias_service =
        match crate::features::server_specific::services::alias_service::AliasService::new(
            "./data/server_specific/targets_alias.json",
        ) {
            Ok(service) => {
                info!("Alias service initialized successfully");
                Some(Arc::new(service))
            }
            Err(e) => {
                error!(
                    "Failed to initialize alias service: {}. Alias functionality will be disabled.",
                    e
                );
                None
            }
        };

    let server_features_orchestrator = Arc::new(ServerFeaturesOrchestrator::new(
        nickname_changer_service.clone(),
        kick_service.clone(),
    ));

    let server_interaction_handler = Arc::new(ServerInteractionHandler::new(
        kick_service,
        voice_interaction_service,
        alias_service,
    ));

    // Create modal storage with 5-minute TTL (plenty of time for users to fill modals)
    let modal_storage = Arc::new(ModalStorage::new(std::time::Duration::from_secs(300)));

    // Spawn background task to clean up expired modal storage entries every minute
    let storage_for_cleanup = modal_storage.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            storage_for_cleanup.cleanup_expired().await;
        }
    });

    let handler = CommandHandler {
        task_service,
        task_orchestrator,
        notification_service,
        timezone_service,
        sqlite_scheduler_repo,
        server_features_orchestrator,
        server_interaction_handler,
        modal_storage,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .register_songbird_with(songbird.clone()) // ← Usar register_songbird_with
        .await?;

    client.start().await?;
    Ok(())
}
