use crate::features::server_specific::config::{NicknameConfig, NicknameTarget, ServerConfig};
use serde_json::json;
use serenity::builder::{CreateAllowedMentions, CreateMessage};
use serenity::http::Http;
use serenity::model::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct NicknameChangerService {
    pub server_config: ServerConfig,
    pub nickname_config: NicknameConfig,
    http: Arc<Http>,
    last_change: std::sync::RwLock<HashMap<u64, std::time::Instant>>,
}

impl NicknameChangerService {
    pub fn new(
        server_config: ServerConfig,
        nickname_config: NicknameConfig,
        http: Arc<Http>,
    ) -> Self {
        Self {
            server_config,
            nickname_config,
            http,
            last_change: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Gets all targets that should have their nicknames changed at the current Argentina time
    pub fn get_scheduled_targets_for_current_time(&self) -> Vec<&NicknameTarget> {
        if !self.nickname_config.is_enabled() {
            return Vec::new();
        }

        self.nickname_config.get_targets_for_current_time()
    }

    /// Changes nickname for a specific user and sends notification message
    pub async fn change_nickname_for_user(&self, user_id: u64) -> Result<String, String> {
        // Check if feature is enabled
        if !self.nickname_config.is_enabled() {
            return Err("Nickname changer feature is disabled".to_string());
        }

        // Check cooldown
        if self.is_in_cooldown(user_id) {
            return Err("User is in cooldown".to_string());
        }

        let target = self.get_target(user_id)?;

        // Get the current name BEFORE the change
        let old_nickname = self
            .get_current_nickname_from_discord(user_id)
            .await
            .unwrap_or_else(|| target.get_current_display_name());

        let new_nickname = target
            .select_random_nickname()
            .ok_or_else(|| format!("No nicknames available for {}", target.display_name))?;

        // FIRST: Change the nickname
        self.update_nickname(user_id, &new_nickname).await?;

        // Small delay to ensure the change is processed
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // SECOND: Send the message with the requested format
        self.send_formatted_message(user_id, &old_nickname, &new_nickname)
            .await?;

        // Finally: Update the internal state
        self.update_state(user_id, &new_nickname);

        Ok(format!(
            "Changed {} from '{}' to '{}'",
            target.display_name, old_nickname, new_nickname
        ))
    }

    // Private helper methods

    fn is_in_cooldown(&self, user_id: u64) -> bool {
        if let Some(last_time) = self.last_change.read().unwrap().get(&user_id) {
            let cooldown =
                std::time::Duration::from_secs(self.nickname_config.cooldown_minutes as u64 * 60);
            return last_time.elapsed() < cooldown;
        }
        false
    }

    fn get_target(&self, user_id: u64) -> Result<&NicknameTarget, String> {
        self.nickname_config
            .find_target(user_id)
            .ok_or_else(|| format!("User {} not found in nickname targets", user_id))
    }

    /// Get the user's current nickname from Discord
    async fn get_current_nickname_from_discord(&self, user_id: u64) -> Option<String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        match self.http.get_member(guild_id, user_id).await {
            Ok(member) => {
                // Prefer the server's nickname, if it does not exist, use the global username
                member
                    .nick
                    .clone()
                    .or_else(|| Some(member.user.name.clone()))
            }
            Err(why) => {
                log::warn!("Failed to get member info for {}: {}", user_id, why);
                None
            }
        }
    }

    async fn update_nickname(&self, user_id: u64, new_nickname: &str) -> Result<(), String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        let map = json!({
            "nick": new_nickname
        });

        match self.http.edit_member(guild_id, user_id, &map, None).await {
            Ok(_) => {
                log::info!(
                    "Successfully updated nickname for user {} to '{}'",
                    user_id,
                    new_nickname
                );
                Ok(())
            }
            Err(why) => {
                let error_msg = format!("Failed to change nickname for user {}: {}", user_id, why);
                log::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    async fn send_formatted_message(
        &self,
        user_id: u64,
        old_nickname: &str,
        new_nickname: &str,
    ) -> Result<(), String> {
        let channel_id = ChannelId::new(self.server_config.general_channel_id);

        // Rol @Miembros
        let miembros_role_id = RoleId::new(1422605245334159401);

        // Base message
        let mut message_content = format!(
            "{}, que bonito nombre tienes... te lo puedo cambiar?\n*{} → {}*",
            old_nickname, old_nickname, new_nickname
        );

        // Emoji ID
        let laugh_emoji = "<:02laugh:923491658010599485>";

        // Extra line when nickname meets condition
        if new_nickname != "Bruja Piruja" && new_nickname != "7 days to cheat" {
            message_content.push_str(&format!(
            "\n<@&{}> Miren todos! {} — digo, {} tiene crisis de identidad! Vamos a reírnos todos de esta humillación {}",
            miembros_role_id.get(),
            old_nickname,
            new_nickname,
            laugh_emoji
        ));
        }

        // Create allowed mentions directly in builder chain (no move error)
        let msg = CreateMessage::new()
            .content(message_content)
            .allowed_mentions(CreateAllowedMentions::new().roles(vec![miembros_role_id]));

        // Send message
        match channel_id.send_message(&self.http, msg).await {
            Ok(_) => {
                log::info!("Formatted message sent successfully for user {}", user_id);
                Ok(())
            }
            Err(why) => {
                let error_msg = format!("Failed to send formatted message: {}", why);
                log::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    fn update_state(&self, user_id: u64, new_nickname: &str) {
        // Update last change timestamp
        self.last_change
            .write()
            .unwrap()
            .insert(user_id, std::time::Instant::now());

        log::debug!(
            "Updated state for user {} with new nickname '{}'",
            user_id,
            new_nickname
        );
    }
}
