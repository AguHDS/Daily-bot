use crate::features::server_specific::config::ServerConfig;
use crate::features::server_specific::config::nickname_config::{NicknameConfig, TargetUser};
use serde_json::json;
use serenity::builder::{CreateAllowedMentions, CreateMessage};
use serenity::http::Http;
use serenity::model::prelude::*;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct NicknameChangerService {
    pub server_config: ServerConfig,
    pub nickname_config: NicknameConfig,
    pub nicknames_pool: Vec<String>,
    http: Arc<Http>,
}

impl NicknameChangerService {
    pub fn new(
        server_config: ServerConfig,
        nickname_config: NicknameConfig,
        nicknames_pool: Vec<String>,
        http: Arc<Http>,
    ) -> Self {
        Self {
            server_config,
            nickname_config,
            nicknames_pool,
            http,
        }
    }

    /// Gets targets that should change nickname based on random probability
    pub fn get_targets_for_random_change(&self) -> Vec<&TargetUser> {
        if !self.nickname_config.is_enabled() {
            return Vec::new();
        }

        self.nickname_config
            .targets
            .iter()
            .filter(|target| target.should_change_nickname(&self.nickname_config.random_config))
            .collect()
    }

    pub async fn change_nickname_for_user(&self, user_id: u64) -> Result<String, String> {
        if !self.nickname_config.is_enabled() {
            return Err("Nickname changer feature is disabled".to_string());
        }

        // Get immutable target for validation
        let target = self.get_target(user_id)?;

        // Check if target can be changed
        if !target.can_change_nickname(&self.nickname_config.random_config) {
            return Err(format!("User {} is in cooldown", target.display_name));
        }

        let old_nickname = self
            .get_current_nickname_from_discord(user_id)
            .await
            .unwrap_or_else(|| target.display_name.clone());

        let new_nickname = self
            .select_random_nickname()
            .ok_or_else(|| format!("No nicknames available for {}", target.display_name))?;

        info!(
            "Changing nickname for {} from '{}' to '{}'",
            target.display_name, old_nickname, new_nickname
        );

        // Update nickname on Discord
        self.update_nickname(user_id, &new_nickname).await?;

        // Small delay to ensure change is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Send notification message
        self.send_formatted_message(&old_nickname, &new_nickname)
            .await?;

        Ok(format!(
            "Successfully changed {} from '{}' to '{}'",
            target.display_name, old_nickname, new_nickname
        ))
    }

    /// Selects a random nickname from the pool
    fn select_random_nickname(&self) -> Option<String> {
        use rand::seq::SliceRandom;
        self.nicknames_pool.choose(&mut rand::thread_rng()).cloned()
    }

    fn get_target(&self, user_id: u64) -> Result<&TargetUser, String> {
        self.nickname_config
            .find_target(user_id)
            .ok_or_else(|| format!("User {} not found in nickname targets", user_id))
    }

    async fn get_current_nickname_from_discord(&self, user_id: u64) -> Option<String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        match self.http.get_member(guild_id, user_id).await {
            Ok(member) => {
                let current_nickname = match &member.nick {
                    Some(nick) => nick.clone(),
                    None => member.user.name.clone(),
                };

                Some(current_nickname)
            }
            Err(why) => {
                warn!("Failed to get member info for {}: {}", user_id, why);
                None
            }
        }
    }

    async fn update_nickname(&self, user_id: u64, new_nickname: &str) -> Result<(), String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        let map = json!({ "nick": new_nickname });

        match self.http.edit_member(guild_id, user_id, &map, None).await {
            Ok(_) => {
                info!(
                    "Successfully updated nickname for user {} to '{}'",
                    user_id, new_nickname
                );
                Ok(())
            }
            Err(why) => {
                let msg = format!("Failed to change nickname for user {}: {}", user_id, why);
                error!("{}", msg);
                Err(msg)
            }
        }
    }

    async fn send_formatted_message(
        &self,
        old_nickname: &str,
        new_nickname: &str,
    ) -> Result<(), String> {
        let channel_id = ChannelId::new(self.server_config.general_channel_id);

        let message_content = format!(
            "{}, que bonito nombre tienes... te lo puedo cambiar?\n*{} → {}*",
            old_nickname, old_nickname, new_nickname
        );

        let msg = CreateMessage::new()
            .content(message_content)
            .allowed_mentions(CreateAllowedMentions::new().empty_roles());

        match channel_id.send_message(&self.http, msg).await {
            Ok(_) => {
                info!("Formatted message sent successfully");
                Ok(())
            }
            Err(why) => {
                let msg = format!("Failed to send formatted message: {}", why);
                error!("{}", msg);
                Err(msg)
            }
        }
    }
}
