use crate::features::server_specific::config::voice_interaction_config::VoiceInteractionConfig;
use serenity::all::{ChannelId, GuildId, UserId};
use serenity::http::Http;
use songbird::Songbird;
use std::sync::Arc;
use tracing::info;

pub struct VoiceInteractionService {
    config: VoiceInteractionConfig,
    http: Arc<Http>,
    songbird: Arc<Songbird>,
}

impl VoiceInteractionService {
    pub fn new(config: VoiceInteractionConfig, http: Arc<Http>, songbird: Arc<Songbird>) -> Self {
        Self {
            config,
            http,
            songbird,
        }
    }

    /// Check if user has permission to use voice interaction commands
    pub fn has_permission(&self, user_id: u64) -> bool {
        self.config.is_user_allowed(user_id)
    }

    /// Check if user has permission to request kicks
    pub fn can_kick(&self, user_id: u64) -> bool {
        self.config.can_user_kick(user_id)
    }

    /// Execute voice action (mute/disconnect) on target user
    pub async fn execute_voice_action(
        &self,
        guild_id: GuildId,
        target_user_id: u64,
        voice_channel_id: ChannelId,
        action: VoiceAction,
    ) -> Result<(), String> {
        let target_user_id = UserId::new(target_user_id);

        // Join the user's voice channel
        self.join_voice_channel(guild_id, voice_channel_id).await?;

        // Execute the action
        match action {
            VoiceAction::Mute => self.mute_user(guild_id, target_user_id).await,
            VoiceAction::Disconnect => self.disconnect_user(guild_id, target_user_id).await,
            VoiceAction::Kick => return Err("Kick action not supported in voice".to_string()),
        }?;

        // wait a moment before leaving
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        self.leave_voice_channel(guild_id).await?;

        Ok(())
    }

    async fn join_voice_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<(), String> {
        let _call = self
            .songbird
            .join(guild_id, channel_id)
            .await
            .map_err(|e| format!("Failed to join voice channel: {}", e))?;

        info!("Joined voice channel {} in guild {}", channel_id, guild_id);
        Ok(())
    }

    async fn leave_voice_channel(&self, guild_id: GuildId) -> Result<(), String> {
        self.songbird
            .leave(guild_id)
            .await
            .map_err(|e| format!("Failed to leave voice channel: {}", e))?;

        info!("Left voice channel in guild {}", guild_id);
        Ok(())
    }

    async fn mute_user(&self, guild_id: GuildId, user_id: UserId) -> Result<(), String> {
        self.http
            .edit_member(
                guild_id,
                user_id,
                &serde_json::json!({ "mute": true }),
                None,
            )
            .await
            .map_err(|e| format!("Failed to mute user: {}", e))?;
        info!("Muted user {} in guild {}", user_id, guild_id);
        Ok(())
    }

    async fn disconnect_user(&self, guild_id: GuildId, user_id: UserId) -> Result<(), String> {
        self.http
            .edit_member(
                guild_id,
                user_id,
                &serde_json::json!({ "channel_id": serde_json::Value::Null }),
                None,
            )
            .await
            .map_err(|e| format!("Failed to disconnect user: {}", e))?;
        info!(
            "Disconnected user {} from voice in guild {}",
            user_id, guild_id
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VoiceAction {
    Mute,
    Disconnect,
    Kick,
}
