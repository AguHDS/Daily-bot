use crate::features::server_specific::config::kick_config::KickConfig;
use crate::features::server_specific::config::nickname_config::NicknameConfig;
use crate::features::server_specific::config::voice_interaction_config::VoiceInteractionConfig;
use crate::features::server_specific::config::{Feature, server_config::ServerConfig};
use crate::features::server_specific::services::{
    kick_service::KickService, nickname_changer::NicknameChangerService,
    voice_interaction_service::VoiceInteractionService,
};
use serenity::http::Http;
use songbird::Songbird;
use std::sync::Arc;
use tracing::error;

/// Initializes server-specific services
pub async fn initialize_specific_services(
    token: &str,
    songbird: Arc<Songbird>,
) -> (
    Option<Arc<NicknameChangerService>>,
    Option<Arc<KickService>>,
    Option<Arc<VoiceInteractionService>>,
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

    // Initialize voice interaction service
    let voice_interaction_service = if server_config
        .enabled_features
        .contains(&Feature::MentionResponse)
    {
        match initialize_voice_interaction_service(token, songbird).await {
            Ok(service) => Some(service),
            Err(e) => {
                error!("Failed to initialize voice interaction service: {}", e);
                None
            }
        }
    } else {
        None
    };

    (nickname_service, kick_service, voice_interaction_service)
}

/// Initializes nickname changer service with configuration
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

/// Initializes kick service with configuration
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

/// Initializes voice interaction service with configuration
async fn initialize_voice_interaction_service(
    token: &str,
    songbird: Arc<Songbird>,
) -> Result<Arc<VoiceInteractionService>, Box<dyn std::error::Error>> {
    let voice_config = VoiceInteractionConfig::load()?;

    Ok(Arc::new(VoiceInteractionService::new(
        voice_config,
        Arc::new(Http::new(token)),
        songbird,
    )))
}
