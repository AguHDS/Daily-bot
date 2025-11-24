use crate::features::server_specific::scheduler::{
    kick_scheduler::KickScheduler, nickname_scheduler::NicknameScheduler,
};
use crate::features::server_specific::services::{
    kick_service::KickService, nickname_changer::NicknameChangerService,
};

use serenity::all::GuildId;
use std::sync::Arc;
use tracing::info;

pub struct ServerFeaturesOrchestrator {
    pub nickname_changer_service: Option<Arc<NicknameChangerService>>,
    pub kick_service: Option<Arc<KickService>>,
}

impl ServerFeaturesOrchestrator {
    pub fn new(
        nickname_changer_service: Option<Arc<NicknameChangerService>>,
        kick_service: Option<Arc<KickService>>,
    ) -> Self {
        Self {
            nickname_changer_service,
            kick_service,
        }
    }

    /// Initialize specific feature for specific server
    pub async fn initialize_server_features(&self, guild_id: GuildId) {
        let my_server_id = 479788664876957737; // My server (Kutums)

        if guild_id.get() != my_server_id {
            return;
        }

        info!(
            "Initializing specific server features for guild {}",
            guild_id
        );

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
}
