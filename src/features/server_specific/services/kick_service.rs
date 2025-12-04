use crate::features::server_specific::config::ServerConfig;
use crate::features::server_specific::config::kick_config::{KickConfig, KickTargetUser};

use serenity::all::{
    ButtonStyle, ChannelId, CreateActionRow, CreateButton, CreateMessage, GuildId, UserId,
};
use serenity::http::Http;

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;
use tracing::error;

pub struct KickService {
    pub server_config: ServerConfig,
    pub kick_config: KickConfig,
    http: Arc<Http>,
}

impl KickService {
    pub fn new(server_config: ServerConfig, kick_config: KickConfig, http: Arc<Http>) -> Self {
        Self {
            server_config,
            kick_config,
            http,
        }
    }

    /// Gets targets that should be considered for kicking based on random probability
    /// Only one user will be returned at most per cycle, even if multiple users pass the probability check
    pub fn get_targets_for_random_kick(&self) -> Vec<&KickTargetUser> {
        if !self.kick_config.is_enabled() {
            return Vec::new();
        }

        // Filter users who can be kicked (not in cooldown)
        let mut eligible_targets: Vec<&KickTargetUser> = self
            .kick_config
            .targets
            .iter()
            .filter(|target| target.can_be_kicked(&self.kick_config.random_config))
            .collect();

        if eligible_targets.is_empty() {
            return Vec::new();
        }

        // Randomize the order so everyone has an equal chance of being selected first
        let mut rng = thread_rng();
        eligible_targets.shuffle(&mut rng);

        // Evaluate each user in random order until one meets the probability
        for target in eligible_targets {
            if target.should_kick(&self.kick_config.random_config) {
                // Only return ONE user at most
                return vec![target];
            }
        }

        // If no user met the probability, return empty
        Vec::new()
    }

    pub async fn send_kick_poll_for_user(&self, user_id: u64) -> Result<String, String> {
        if !self.kick_config.is_enabled() {
            return Err("Kick feature is disabled".to_string());
        }

        let target = self.get_target(user_id)?;

        if !target.can_be_kicked(&self.kick_config.random_config) {
            return Err(format!("User {} is in cooldown", target.display_name));
        }

        // Get server name instead of global name
        let server_name = self
            .get_user_server_name(user_id)
            .await
            .unwrap_or_else(|| target.display_name.clone());

        self.send_kick_poll_message(&server_name).await?;

        Ok(format!("Sent kick poll for user {}", server_name))
    }

    fn get_target(&self, user_id: u64) -> Result<&KickTargetUser, String> {
        self.kick_config
            .find_target(user_id)
            .ok_or_else(|| format!("User {} not found in kick targets", user_id))
    }

    /// Gets the name of the user on the server (nickname or display name)
    async fn get_user_server_name(&self, user_id: u64) -> Option<String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        // Get nickname from the guild
        match guild_id.member(&self.http, user_id).await {
            Ok(member) => {
                // Use the nickname if it exists, but use the global display name
                member
                    .nick
                    .clone()
                    .or_else(|| Some(member.user.name.clone()))
            }
            Err(_) => {
                error!("Failed to get member {} from guild {}", user_id, guild_id);
                None
            }
        }
    }

    async fn send_kick_poll_message(&self, display_name: &str) -> Result<(), String> {
        let channel_id = ChannelId::new(self.server_config.general_channel_id);

        let message_content = format!("Puedo kickear a {}?", display_name);

        let yes_button = CreateButton::new("kick_yes")
            .label("Sí")
            .style(ButtonStyle::Success);

        let no_button = CreateButton::new("kick_no")
            .label("No")
            .style(ButtonStyle::Danger);

        let action_row = CreateActionRow::Buttons(vec![yes_button, no_button]);

        let msg = CreateMessage::new()
            .content(message_content)
            .components(vec![action_row]);

        match channel_id.send_message(&self.http, msg).await {
            Ok(_) => Ok(()),
            Err(why) => {
                let msg = format!("Failed to send kick poll message: {}", why);
                error!("{}", msg);
                Err(msg)
            }
        }
    }

    /// Actual kick execution (to be called when button is pressed)
    pub async fn execute_kick(&self, user_id: u64) -> Result<(), String> {
        let guild_id = GuildId::new(self.server_config.server_id);
        let user_id = UserId::new(user_id);

        match self.http.kick_member(guild_id, user_id, None).await {
            Ok(_) => Ok(()),
            Err(why) => {
                let msg = format!("Failed to kick user {}: {}", user_id, why);
                error!("{}", msg);
                Err(msg)
            }
        }
    }
}
