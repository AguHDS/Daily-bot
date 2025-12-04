use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickRandomConfig {
    pub check_interval_minutes: u32,
    pub kick_probability: f32,
    pub min_minutes_between_kicks: u32,
}

impl Default for KickRandomConfig {
    fn default() -> Self {
        Self {
            check_interval_minutes: 360,
            kick_probability: 0.0,
            min_minutes_between_kicks: 720,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickTargetUser {
    pub user_id: u64,
    pub display_name: String,
    pub kick_probability: Option<f32>,
    pub last_kick_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KickConfig {
    pub enabled: bool,
    pub random_config: KickRandomConfig,
    pub targets: Vec<KickTargetUser>,
}

impl KickConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let targets_path = "./data/server_specific/kick_targets.json";
        let content = fs::read_to_string(targets_path)?;
        let config: KickConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let targets_path = "./data/server_specific/kick_targets.json";
        let content = serde_json::to_string_pretty(self)?;
        fs::write(targets_path, content)?;
        Ok(())
    }

    pub fn find_target(&self, user_id: u64) -> Option<&KickTargetUser> {
        self.targets.iter().find(|target| target.user_id == user_id)
    }

    pub fn find_target_mut(&mut self, user_id: u64) -> Option<&mut KickTargetUser> {
        self.targets
            .iter_mut()
            .find(|target| target.user_id == user_id)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl KickTargetUser {
    pub fn can_be_kicked(&self, config: &KickRandomConfig) -> bool {
        if let Some(last_kick) = self.last_kick_time {
            let min_interval = Duration::minutes(config.min_minutes_between_kicks as i64);
            let now = Utc::now();
            return now - last_kick >= min_interval;
        }
        true
    }

    pub fn should_kick(&self, config: &KickRandomConfig) -> bool {
        if !self.can_be_kicked(config) {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        let probability = self.kick_probability.unwrap_or(config.kick_probability);
        rng.gen_bool(probability as f64)
    }

    pub fn update_kick_time(&mut self) {
        self.last_kick_time = Some(Utc::now());
    }
}
