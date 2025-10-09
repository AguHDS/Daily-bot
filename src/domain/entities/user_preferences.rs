use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: u64,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserPreferences {
    pub fn new(user_id: u64, timezone: String) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            timezone,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_timezone(&mut self, new_timezone: String) {
        self.timezone = new_timezone;
        self.updated_at = Utc::now();
    }

    pub fn is_valid(&self) -> bool {
        !self.timezone.is_empty() && self.user_id > 0
    }
}

impl PartialEq for UserPreferences {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}