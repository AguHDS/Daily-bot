use crate::features::server_specific::config::alias_config::{AliasConfig, UserAlias};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

pub struct AliasService {
    config: Arc<RwLock<AliasConfig>>,
    config_path: String,
}

impl AliasService {
    /// Create a new AliasService
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = match AliasConfig::load(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(
                    "Failed to load alias config from {}: {}. Using default.",
                    config_path, e
                );
                AliasConfig::default()
            }
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.to_string(),
        })
    }

    /// Find user by alias name
    pub async fn find_user_by_alias(&self, alias_name: &str) -> Option<UserAlias> {
        let config = self.config.read().await;
        config.find_user_by_alias(alias_name).cloned()
    }

    /// Find user by user_id
    pub async fn find_user_by_id(&self, user_id: u64) -> Option<UserAlias> {
        let config = self.config.read().await;
        config.find_user_by_id(user_id).cloned()
    }

    /// Add a new alias for a user
    pub async fn add_alias(&self, user_id: u64, discord_username: &str, new_alias: &str) -> bool {
        let mut config = self.config.write().await;

        // Check if user already exists
        if let Some(_user) = config.users.iter_mut().find(|u| u.user_id == user_id) {
            // User exists, add alias
            if config.add_alias(user_id, new_alias) {
                debug!("Added alias '{}' for user {}", new_alias, user_id);

                // Save config
                if let Err(e) = config.save(&self.config_path) {
                    error!("Failed to save alias config: {}", e);
                }
                return true;
            }
        } else {
            // User doesn't exist, create new entry
            let user_alias = UserAlias {
                user_id,
                discord_username: discord_username.to_string(),
                alias: vec![new_alias.to_string()],
            };
            config.users.push(user_alias);

            debug!(
                "Created new user entry with alias '{}' for user {}",
                new_alias, user_id
            );

            // Save config
            if let Err(e) = config.save(&self.config_path) {
                error!("Failed to save alias config: {}", e);
            }
            return true;
        }

        false
    }

    /// Remove an alias from a user
    pub async fn remove_alias(&self, user_id: u64, alias_to_remove: &str) -> bool {
        let mut config = self.config.write().await;

        if config.remove_alias(user_id, alias_to_remove) {
            debug!("Removed alias '{}' from user {}", alias_to_remove, user_id);

            // Save config
            if let Err(e) = config.save(&self.config_path) {
                error!("Failed to save alias config: {}", e);
            }
            return true;
        }

        false
    }

    /// Get all users with their aliases
    pub async fn get_all_users(&self) -> Vec<UserAlias> {
        let config = self.config.read().await;
        config.users.clone()
    }

    /// Extract username from message content using aliases
    pub async fn extract_username_from_content(&self, content: &str) -> Option<String> {
        let words: Vec<&str> = content.split_whitespace().collect();

        for word in words {
            // Skip if word is too short or contains special characters that indicate it's not a name
            if word.len() < 2 || word.contains('@') || word.contains('#') || word.contains(':') {
                continue;
            }

            // Clean the word (remove punctuation)
            let cleaned_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if cleaned_word.len() < 2 {
                continue;
            }

            // Check if this word matches any alias
            if let Some(user) = self.find_user_by_alias(cleaned_word).await {
                return Some(user.discord_username);
            }
        }

        None
    }

    /// Extract user_id from message content using aliases
    pub async fn extract_user_id_from_content(&self, content: &str) -> Option<u64> {
        let words: Vec<&str> = content.split_whitespace().collect();

        for word in words {
            // Skip if word is too short or contains special characters that indicate it's not a name
            if word.len() < 2 || word.contains('@') || word.contains('#') || word.contains(':') {
                continue;
            }

            // Clean the word (remove punctuation)
            let cleaned_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if cleaned_word.len() < 2 {
                continue;
            }

            // Check if this word matches any alias
            if let Some(user) = self.find_user_by_alias(cleaned_word).await {
                return Some(user.user_id);
            }
        }

        None
    }
}
