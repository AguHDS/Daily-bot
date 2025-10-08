use crate::domain::repositories::ConfigRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConfigService {
    config_repo: Arc<dyn ConfigRepository>,
}

impl ConfigService {
    pub fn new(config_repo: Arc<dyn ConfigRepository>) -> Self {
        Self { config_repo }
    }

    pub async fn set_notification_channel(
        &self,
        guild_id: u64,
        channel_id: u64,
    ) -> Result<(), String> {
        // Validaciones de negocio podrían ir aquí
        if guild_id == 0 {
            return Err("Invalid guild ID".to_string());
        }

        if channel_id == 0 {
            return Err("Invalid channel ID".to_string());
        }

        self.config_repo
            .set_notification_channel(guild_id, channel_id);
        Ok(())
    }

    pub async fn get_notification_channel(&self, guild_id: u64) -> Option<u64> {
        self.config_repo.get_notification_channel(guild_id)
    }

    pub async fn validate_guild_context(&self, guild_id: Option<u64>) -> Result<u64, String> {
        guild_id.ok_or_else(|| "This command can only be used in a server".to_string())
    }
}
