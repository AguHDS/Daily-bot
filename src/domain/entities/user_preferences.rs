use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: u64,
    pub timezone: String,
    pub date_format: Option<String>, // "YMD", "DMY", or "MDY"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserPreferences {
    pub fn new_with_format(user_id: u64, timezone: String, date_format: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            timezone,
            date_format,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_timezone_and_format(
        &mut self,
        new_timezone: String,
        date_format: Option<String>,
    ) {
        self.timezone = new_timezone;
        self.date_format = date_format;
        self.updated_at = Utc::now();
    }

    pub fn is_valid(&self) -> bool {
        !self.timezone.is_empty() && self.user_id > 0
    }

    /// Get the date format placeholder pattern for UI display
    pub fn get_date_format_placeholder(&self) -> &'static str {
        match self.date_format.as_deref() {
            Some("DMY") => "DD-MM-YYYY",
            Some("MDY") => "MM-DD-YYYY",
            Some("YMD") | None => "YYYY-MM-DD", // Default to YMD if not set
            _ => "YYYY-MM-DD", // Fallback
        }
    }
}

impl PartialEq for UserPreferences {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}
