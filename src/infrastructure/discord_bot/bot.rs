use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::repositories::{ConfigRepository, TaskRepository, UserPreferencesRepository};
use crate::infrastructure::repositories::json_config_repository::JsonConfigRepository;
use crate::infrastructure::repositories::{
    json_task_repository::JsonTaskRepository,
    json_user_preferences_repository::JsonUserPreferencesRepository,
};
use crate::infrastructure::scheduler::scheduler_tokio::start_scheduler;
use crate::infrastructure::timezone::timezone_manager::TimezoneManager;
use serenity::all::Message;
use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;

use crate::features::server_specific::{
    Feature, MessageHandler, NicknameChangerService, NicknameConfig, NicknameScheduler,
    ServerConfig,
};
use serenity::http::Http;

pub struct CommandHandler {
    pub task_service: Arc<TaskService>,
    pub config_service: Arc<ConfigService>,
    pub notification_service: Arc<NotificationService>,
    pub timezone_service: Arc<TimezoneService>,
    pub nickname_changer_service: Option<Arc<NicknameChangerService>>,
    pub message_handler: Option<Arc<MessageHandler>>,
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        // register commands
        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;

            let commands = vec![
            crate::application::commands::register_add_task_command(),
            crate::application::commands::register_list_tasks_command(),
            crate::application::commands::register_remove_task_command(),
            crate::application::commands::register_help_command(),
            crate::application::commands::edit_task::register_edit_task_command(),
            crate::application::commands::set_notification_channel::register_set_notification_channel_command(),
            crate::application::commands::timezone::register_timezone_command(),
        ];

            if let Err(e) = guild_id.set_commands(&ctx.http, commands).await {
                eprintln!("Failed to set commands for guild {}: {}", guild_id, e);
            } else {
                println!("Commands updated for guild {}", guild_id);
            }

            self.initialize_joke_features(&ctx, guild_id).await;
        }

        // Start scheduler with services
        start_scheduler(
            Arc::new(ctx.clone()),
            self.task_service.clone(),
            self.config_service.clone(),
            self.notification_service.clone(),
        );
        println!("Scheduler started");
    }

    /// Decide what to do depending on user's interaction type with the bot
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                "add_task" => {
                    crate::application::commands::add_task::run_add_task(
                        &ctx,
                        command,
                        &self.task_service,
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
                        &self.task_service,
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
                    &self.task_service,
                    &self.timezone_service,
                )
                .await;
            }
            Interaction::Modal(_) => {
                crate::application::commands::interaction_handlers::handle_modal(
                    &ctx,
                    &interaction,
                    &self.task_service,
                    &self.timezone_service,
                )
                .await;
            }
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Pass the message to our message_handler if it exists
        if let Some(handler) = &self.message_handler {
            handler.handle_message(&ctx, &msg).await;
        }
    }
}

impl CommandHandler {
    async fn initialize_joke_features(&self, ctx: &Context, guild_id: GuildId) {
        let my_server_id = 479788664876957737; // HERAMNOS KUTUM

        if guild_id.get() != my_server_id {
            return;
        }

        println!("Initializing joke features for guild {}", guild_id);

        // initialize nickname scheduler if the service exists
        if let Some(nickname_service) = &self.nickname_changer_service {
            // Update the service with the real HTTP client
            let updated_service = Arc::new(NicknameChangerService::new(
                nickname_service.server_config.clone(),
                nickname_service.nickname_config.clone(),
                ctx.http.clone(),
            ));

            let scheduler = NicknameScheduler::new(updated_service.clone());
            scheduler.start().await;
        }
    }
}

/// Factory that initializes repositories, services, and the discord bot, and then boots it
pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT;

    // initialize repositories
    let task_repo: Arc<dyn TaskRepository> = Arc::new(JsonTaskRepository::new("./data/tasks.json"));
    let config_repo: Arc<dyn ConfigRepository> = Arc::new(JsonConfigRepository::new(
        "./data/channel_notification.json",
    ));
    let user_prefs_repo: Arc<dyn UserPreferencesRepository> = Arc::new(
        JsonUserPreferencesRepository::new("./data/user_timezone_config.json"),
    );

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

    let (nickname_changer_service, message_handler) = initialize_joke_services().await;

    let handler = CommandHandler {
        task_service: task_service.clone(),
        config_service: config_service.clone(),
        notification_service: notification_service.clone(),
        timezone_service: timezone_service.clone(),
        nickname_changer_service,
        message_handler,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    println!("Bot starting...");
    client.start().await?;
    Ok(())
}

// ==================== FUNCTION TO INITIALIZE HERMANOS KUTUM LOGIC ====================
async fn initialize_joke_services() -> (
    Option<Arc<NicknameChangerService>>,
    Option<Arc<MessageHandler>>,
) {
    let server_config = ServerConfig::my_server();

    // Check if features are enabled
    if !server_config
        .enabled_features
        .contains(&Feature::NicknameChanger)
        && !server_config
            .enabled_features
            .contains(&Feature::MentionResponse)
    {
        return (None, None);
    }

    let nickname_service = if server_config
        .enabled_features
        .contains(&Feature::NicknameChanger)
    {
        let nickname_config = NicknameConfig::default_targets();
        Some(Arc::new(NicknameChangerService::new(
            server_config.clone(),
            nickname_config,
            // Placeholder - will be updated with the actual HTTP in initialize_joke_features
            Arc::new(Http::new("")),
        )))
    } else {
        None
    };

    let message_handler = if server_config
        .enabled_features
        .contains(&Feature::MentionResponse)
    {
        Some(Arc::new(MessageHandler::new(server_config)))
    } else {
        None
    };

    (nickname_service, message_handler)
}
