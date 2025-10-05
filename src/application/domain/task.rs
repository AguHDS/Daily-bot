use chrono::{DateTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub user_id: u64,
    pub message: String,
    pub scheduled_time: Option<DateTime<Utc>>, // initial scheduled time for task
    pub recurrence: Option<Recurrence>,
    pub notification_method: NotificationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Recurrence {
    Weekly {
        days: Vec<Weekday>,
        hour: u8,
        minute: u8,
    },
    EveryXDays {
        interval: u32,
        hour: u8,
        minute: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationMethod {
    DM,
    Channel,
    Both,
}

impl Task {
    pub fn new(
        id: u64,
        user_id: u64,
        message: String,
        scheduled_time: Option<DateTime<Utc>>,
        recurrence: Option<Recurrence>,
        notification_method: NotificationMethod,
    ) -> Self {
        Self {
            id,
            user_id,
            message,
            scheduled_time,
            recurrence,
            notification_method,
        }
    }
}
