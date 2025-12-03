use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAlias {
    pub user_id: u64,
    pub discord_username: String,
    pub alias: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasConfig {
    pub users: Vec<UserAlias>,
}

impl AliasConfig {
    /// Load alias configuration from JSON file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AliasConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save alias configuration to JSON file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Find user by alias
    pub fn find_user_by_alias(&self, alias_name: &str) -> Option<&UserAlias> {
        let alias_lower = alias_name.to_lowercase();
        
        self.users.iter().find(|user| {
            // Check discord username
            user.discord_username.to_lowercase() == alias_lower ||
            // Check user_id as string
            user.user_id.to_string() == alias_lower ||
            // Check any alias
            user.alias.iter().any(|a| a.to_lowercase() == alias_lower)
        })
    }

    /// Find user by user_id
    pub fn find_user_by_id(&self, user_id: u64) -> Option<&UserAlias> {
        self.users.iter().find(|user| user.user_id == user_id)
    }

    /// Add a new alias for a user
    pub fn add_alias(&mut self, user_id: u64, new_alias: &str) -> bool {
        if let Some(user) = self.users.iter_mut().find(|u| u.user_id == user_id) {
            let alias_lower = new_alias.to_lowercase();
            
            // Check if alias already exists
            if !user.alias.iter().any(|a| a.to_lowercase() == alias_lower) {
                user.alias.push(new_alias.to_string());
                return true;
            }
        }
        false
    }

    /// Remove an alias from a user
    pub fn remove_alias(&mut self, user_id: u64, alias_to_remove: &str) -> bool {
        if let Some(user) = self.users.iter_mut().find(|u| u.user_id == user_id) {
            let alias_lower = alias_to_remove.to_lowercase();
            let original_len = user.alias.len();
            
            user.alias.retain(|a| a.to_lowercase() != alias_lower);
            
            return user.alias.len() < original_len;
        }
        false
    }
}

impl Default for AliasConfig {
    fn default() -> Self {
        Self { users: Vec::new() }
    }
}