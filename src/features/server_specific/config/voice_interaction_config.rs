use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionTarget {
    pub user_id: u64,
    pub display_name: String,
    #[serde(default = "default_kick_permission")] // Default to false if not specified
    pub kick_request_permission: bool,
}

// Default function for serde
fn default_kick_permission() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInteractionConfig {
    pub targets: Vec<PermissionTarget>,
}

impl VoiceInteractionConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "src/features/server_specific/data/interaction_permission.json";
        let config_content = fs::read_to_string(config_path)?;
        let config: VoiceInteractionConfig = serde_json::from_str(&config_content)?;
        Ok(config)
    }

    pub fn is_user_allowed(&self, user_id: u64) -> bool {
        self.targets.iter().any(|target| target.user_id == user_id)
    }

    pub fn can_user_kick(&self, user_id: u64) -> bool {
        self.targets
            .iter()
            .find(|target| target.user_id == user_id)
            .map(|target| target.kick_request_permission)
            .unwrap_or(false)
    }

    pub fn find_target(&self, user_id: u64) -> Option<&PermissionTarget> {
        self.targets.iter().find(|target| target.user_id == user_id)
    }
}
