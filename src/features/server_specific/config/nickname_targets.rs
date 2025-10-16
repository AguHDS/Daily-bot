use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicknameTarget {
    pub user_id: u64,
    pub display_name: String,
    pub schedules: Vec<String>,
    pub nickname_pool: Vec<String>,
    pub last_used_nickname: Option<String>,
}

impl NicknameTarget {
    pub fn new(
        user_id: u64,
        display_name: String,
        schedules: Vec<String>,
        nickname_pool: Vec<String>,
    ) -> Self {
        Self {
            user_id,
            display_name,
            schedules,
            nickname_pool,
            last_used_nickname: None,
        }
    }

    /// Gets the current time in Argentina (UTC-3)
    pub fn get_argentina_time() -> DateTime<FixedOffset> {
        // Argentina is at UTC-3 (and does not use daylight saving time in most provinces)
        let argentina_offset = FixedOffset::west_opt(3 * 3600).unwrap();

        // Convert current UTC time to Argentina time
        Utc::now().with_timezone(&argentina_offset)
    }

    /// Selects a random nickname from the pool
    pub fn select_random_nickname(&self) -> Option<String> {
        use rand::seq::SliceRandom;
        self.nickname_pool.choose(&mut rand::thread_rng()).cloned()
    }

    /// Gets the current display name (last used nickname or the real name)
    pub fn get_current_display_name(&self) -> String {
        self.last_used_nickname
            .clone()
            .unwrap_or_else(|| self.display_name.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicknameConfig {
    pub targets: Vec<NicknameTarget>,
    pub cooldown_minutes: u32,
    pub enabled: bool,
}

impl Default for NicknameConfig {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            cooldown_minutes: 1,
            enabled: true,
        }
    }
}

impl NicknameConfig {
    /// Default configuration with Argentina timezone
    pub fn default_targets() -> Self {
        Self {
            cooldown_minutes: 0,
            enabled: true,
            targets: vec![NicknameTarget::new(
                348513689974079509, // Dalex
                "dalex512".to_string(),
                vec![
                    "16:34".to_string(),
                    "04:06".to_string(),
                    "04:07".to_string(),
                    "04:08".to_string(),
                    "23:37".to_string(),
                ],
                vec![
                    "Bruja Piruja".to_string(),
                    /* "7 days to cheat".to_string(), */
                    /* "Gorila Power 🦍".to_string(), */
                    /* "Chupa Banana 🍌".to_string(), */
                    /* "Devora Berenjenas 🍆".to_string(), */
                ],
            )],
        }
    }

    /// Finds a target by user_id
    pub fn find_target(&self, user_id: u64) -> Option<&NicknameTarget> {
        self.targets.iter().find(|target| target.user_id == user_id)
    }

    /// Gets all targets that should change nickname at the current time (Argentina time)
    pub fn get_targets_for_current_time(&self) -> Vec<&NicknameTarget> {
        let current_argentina_time = NicknameTarget::get_argentina_time();
        let current_time_str = current_argentina_time.format("%H:%M").to_string();

        self.targets
            .iter()
            .filter(|target| {
                let should_change = target.schedules.contains(&current_time_str);
                should_change
            })
            .collect()
    }

    /// Checks if the feature is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
