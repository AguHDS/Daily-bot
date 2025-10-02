use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub user_id: u64,
    pub message: String,
    pub scheduled_time: Option<DateTime<Utc>>, 
    pub completed: bool,
    pub repeat_daily: bool,
}

impl Task {
    pub fn new(
        id: u64,
        user_id: u64,
        message: String,
        scheduled_time: Option<DateTime<Utc>>,
        repeat_daily: bool,
    ) -> Self {
        Self {
            id,
            user_id,
            message,
            scheduled_time,
            completed: false,
            repeat_daily,
        }
    }
}
