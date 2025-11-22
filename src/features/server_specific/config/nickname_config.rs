use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomConfig {
    pub check_interval_minutes: u32,
    pub change_probability: f32,
    pub min_minutes_between_changes: u32,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            check_interval_minutes: 15,
            change_probability: 0.05,
            min_minutes_between_changes: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetUser {
    pub user_id: u64,
    pub display_name: String,
    pub change_probability: Option<f32>,
    pub last_change_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicknameConfig {
    pub enabled: bool,
    pub random_config: RandomConfig,
    pub targets: Vec<TargetUser>,
}

impl NicknameConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let targets_path = "src/features/server_specific/data/nickname_targets.json";
        let content = fs::read_to_string(targets_path)?;
        let config: NicknameConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn load_nicknames() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let nicknames_path = "src/features/server_specific/data/nicknames.json";
        let content = fs::read_to_string(nicknames_path)?;
        let nicknames: Vec<String> = serde_json::from_str(&content)?;
        Ok(nicknames)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let targets_path = "src/features/server_specific/data/nickname_targets.json";
        let content = serde_json::to_string_pretty(self)?;
        fs::write(targets_path, content)?;
        Ok(())
    }

    pub fn find_target(&self, user_id: u64) -> Option<&TargetUser> {
        self.targets.iter().find(|target| target.user_id == user_id)
    }

    pub fn find_target_mut(&mut self, user_id: u64) -> Option<&mut TargetUser> {
        self.targets.iter_mut().find(|target| target.user_id == user_id)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl TargetUser {
    pub fn can_change_nickname(&self, config: &RandomConfig) -> bool {
        if let Some(last_change) = self.last_change_time {
            let min_interval = Duration::minutes(config.min_minutes_between_changes as i64);
            let now = Utc::now();
            return now - last_change >= min_interval;
        }
        true
    }

    pub fn should_change_nickname(&self, config: &RandomConfig) -> bool {
        if !self.can_change_nickname(config) {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let probability = self.change_probability.unwrap_or(config.change_probability);
        rng.gen_bool(probability as f64)
    }

    pub fn update_change_time(&mut self) {
        self.last_change_time = Some(Utc::now());
    }
}