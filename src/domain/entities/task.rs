use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

// Auxiliary structs for serialization in SQLite repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyRecurrenceData {
    pub days: Vec<Weekday>,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EveryXDaysRecurrenceData {
    pub interval: u32,
    pub hour: u8,
    pub minute: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub user_id: u64,
    pub guild_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_time: Option<DateTime<Utc>>, // next scheduled time for task
    pub recurrence: Option<Recurrence>,
    pub notification_method: NotificationMethod,
    pub channel_id: Option<u64>,
    pub mention: Option<String>, // Optional @user or @role mention for notifications
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
        guild_id: u64,
        title: String,
        description: Option<String>,
        scheduled_time: Option<DateTime<Utc>>,
        recurrence: Option<Recurrence>,
        notification_method: NotificationMethod,
        channel_id: Option<u64>,
        mention: Option<String>,
    ) -> Self {
        Self {
            id,
            user_id,
            guild_id,
            title,
            description,
            scheduled_time,
            recurrence,
            notification_method,
            channel_id,
            mention,
        }
    }

    /// Calculates the next occurrence datetime for a recurring task. Returns `None` if the task is not recurring
    pub fn next_occurrence(&self) -> Option<DateTime<Utc>> {
        match &self.recurrence {
            Some(Recurrence::Weekly { days, hour, minute }) => {
                let now = Utc::now();
                // create today at the specified time
                let today_at_time = now
                    .with_hour(*hour as u32)
                    .and_then(|t| t.with_minute(*minute as u32))
                    .and_then(|t| t.with_second(0))
                    .unwrap();

                let mut candidate = today_at_time;

                // if current time already passed, start from tomorrow
                if candidate <= now {
                    candidate = candidate + Duration::days(1);
                }

                // search for next candidate day
                for _ in 0..7 {
                    if days.contains(&candidate.weekday()) {
                        return Some(candidate);
                    }
                    candidate = candidate + Duration::days(1);
                }
                None
            }
            Some(Recurrence::EveryXDays {
                interval,
                hour,
                minute,
            }) => {
                if let Some(current) = self.scheduled_time {
                    let next = current + Duration::days(*interval as i64);
                    return Some(
                        next.with_hour(*hour as u32)
                            .and_then(|t| t.with_minute(*minute as u32))
                            .and_then(|t| t.with_second(0))
                            .unwrap(),
                    );
                }
                None
            }
            None => None,
        }
    }
}
