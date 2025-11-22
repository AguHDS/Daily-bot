use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_orchestrator::TaskOrchestrator;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{
    ConfigRepository, TaskRepository, TaskSchedulerRepository, UserPreferencesRepository,
};
use crate::features::server_specific::config::ServerConfig;
use crate::features::server_specific::config::kick_config::KickConfig;
use crate::features::server_specific::config::nickname_config::NicknameConfig;
use crate::features::server_specific::{
    Feature, KickScheduler, KickService, NicknameChangerService, NicknameScheduler,
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
use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
    Interaction, Ready, UserId,
};
use serenity::http::Http;
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
    pub kick_service: Option<Arc<KickService>>,
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
        let my_server_id = 1422605167580155914; // My server (Kutums)

        if guild_id.get() != my_server_id {
            return;
        }

        info!("Initializing specific server features for guild {}", guild_id);

        // Start nickname scheduler if the service exists
        if let Some(nickname_service) = &self.nickname_changer_service {
            let scheduler = NicknameScheduler::new(nickname_service.clone());
            scheduler.start().await;
        }

        // Start kick scheduler if the service exists
        if let Some(kick_service) = &self.kick_service {
            let scheduler = KickScheduler::new(kick_service.clone());
            scheduler.start().await;
        }
    }

    async fn handle_kick_buttons(&self, ctx: &Context, component: &ComponentInteraction) {
        let custom_id = &component.data.custom_id;

        match custom_id.as_str() {
            "kick_yes" => {
                self.handle_kick_decision(ctx, component, true).await;
            }
            "kick_no" => {
                self.handle_kick_decision(ctx, component, false).await;
            }
            _ => {
                debug!("Unknown button interaction: {}", custom_id);
            }
        }
    }

    async fn handle_kick_decision(
        &self,
        ctx: &Context,
        component: &ComponentInteraction,
        approved: bool,
    ) {
        let original_message = component.message.content.clone();

        if original_message.is_empty() {
            error!("No content in kick poll message");
            return;
        }

        // Extract username from message
        let server_name = extract_username_from_kick_message(&original_message);

        if let (Some(kick_service), Some(server_name)) = (&self.kick_service, server_name) {
            if approved {
                // Find user ID by server name
                if let Some(target) = self
                    .find_target_by_server_name(&server_name, &ctx.http)
                    .await
                {
                    match kick_service.execute_kick(target.user_id).await {
                        Ok(_) => {
                            let response = format!("{} kickeado.", server_name);
                            let _ = component
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(response)
                                            .components(vec![]),
                                    ),
                                )
                                .await;
                        }
                        Err(e) => {
                            let response = format!("Error al kickear a {}: {}", server_name, e);
                            let _ = component
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(response)
                                            .components(vec![]),
                                    ),
                                )
                                .await;
                            error!("Failed to kick user {}: {}", server_name, e);
                        }
                    }
                } else {
                    let response = format!("No se pudo encontrar al usuario: {}", server_name);
                    let _ = component
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .content(response)
                                    .components(vec![]),
                            ),
                        )
                        .await;
                }
            } else {
                let response = format!("bueno...");
                let _ = component
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content(response)
                                .components(vec![]),
                        ),
                    )
                    .await;
            }
        } else {
            error!("Kick service not available or username not found");
        }
    }

    /// Find target by server name
    async fn find_target_by_server_name(
        &self,
        server_name: &str,
        http: &Http,
    ) -> Option<&crate::features::server_specific::config::kick_config::KickTargetUser> {
        if let Some(kick_service) = &self.kick_service {
            let guild_id = GuildId::new(kick_service.server_config.server_id);

            for target in &kick_service.kick_config.targets {
                let user_id = UserId::new(target.user_id);

                match guild_id.member(http, user_id).await {
                    Ok(member) => {
                        let target_server_name = member
                            .nick
                            .clone()
                            .unwrap_or_else(|| member.user.name.clone());
                        if target_server_name == server_name {
                            return Some(target);
                        }
                    }
                    Err(_) => {
                        if target.display_name == server_name {
                            return Some(target);
                        }
                    }
                }
            }
        }
        None
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
            self.register_commands_for_guild(&ctx, guild.id).await;
            self.initialize_joke_features(guild.id).await;
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
            Interaction::Component(component) => {
                debug!(
                    "Received component interaction: {}",
                    component.data.custom_id
                );

                if component.data.custom_id.starts_with("kick_") {
                    self.handle_kick_buttons(&ctx, component).await;
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

    // Initialize nickname changer and kick services
    let (nickname_changer_service, kick_service) = initialize_joke_services(&token).await;

    let handler = CommandHandler {
        task_service,
        task_orchestrator,
        config_service,
        notification_service,
        timezone_service,
        sqlite_scheduler_repo,
        nickname_changer_service,
        kick_service,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}

// ==================== Additional features for my server ====================

async fn initialize_joke_services(
    token: &str,
) -> (
    Option<Arc<NicknameChangerService>>,
    Option<Arc<KickService>>,
) {
    let server_config = ServerConfig::my_server();

    // Initialize nickname changer service
    let nickname_service = if server_config
        .enabled_features
        .contains(&Feature::NicknameChanger)
    {
        match initialize_nickname_service(&server_config, token).await {
            Ok(service) => Some(service),
            Err(e) => {
                error!("Failed to initialize nickname service: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize kick service
    let kick_service = if server_config.enabled_features.contains(&Feature::Kick) {
        match initialize_kick_service(&server_config, token).await {
            Ok(service) => Some(service),
            Err(e) => {
                error!("Failed to initialize kick service: {}", e);
                None
            }
        }
    } else {
        None
    };

    (nickname_service, kick_service)
}

async fn initialize_nickname_service(
    server_config: &ServerConfig,
    token: &str,
) -> Result<Arc<NicknameChangerService>, Box<dyn std::error::Error>> {
    let nickname_config = NicknameConfig::load()?;
    let nicknames_pool = NicknameConfig::load_nicknames()?;

    if nicknames_pool.is_empty() {
        return Err("No nicknames available in the pool".into());
    }

    Ok(Arc::new(NicknameChangerService::new(
        server_config.clone(),
        nickname_config,
        nicknames_pool,
        Arc::new(Http::new(token)),
    )))
}

async fn initialize_kick_service(
    server_config: &ServerConfig,
    token: &str,
) -> Result<Arc<KickService>, Box<dyn std::error::Error>> {
    let kick_config = KickConfig::load()?;

    Ok(Arc::new(KickService::new(
        server_config.clone(),
        kick_config,
        Arc::new(Http::new(token)),
    )))
}

// ==================== Helper functions ====================

/// Extract username from kick message: "Puedo kickear a username?"
fn extract_username_from_kick_message(message: &str) -> Option<String> {
    let prefix = "Puedo kickear a ";
    let suffix = "?";

    if let Some(start) = message.find(prefix) {
        let start_idx = start + prefix.len();
        if let Some(end) = message.find(suffix) {
            if end > start_idx {
                return Some(message[start_idx..end].to_string());
            }
        }
    }
    None
}
