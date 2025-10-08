use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub user_id: u64,
    pub guild_id: u64,
    pub message: String,
    pub scheduled_time: Option<DateTime<Utc>>, // next scheduled time for task
    pub recurrence: Option<Recurrence>,
    pub notification_method: NotificationMethod,
    pub channel_id: Option<u64>,
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
        message: String,
        scheduled_time: Option<DateTime<Utc>>,
        recurrence: Option<Recurrence>,
        notification_method: NotificationMethod,
        channel_id: Option<u64>,
    ) -> Self {
        Self {
            id,
            user_id,
            guild_id,
            message,
            scheduled_time,
            recurrence,
            notification_method,
            channel_id,
        }
    }

    /// Calculates the next occurrence datetime for a recurring task. Returns `None` if the task is not recurring
    pub fn next_occurrence(&self) -> Option<DateTime<Utc>> {
        match &self.recurrence {
            Some(Recurrence::Weekly { days, hour, minute }) => {
                let now = Utc::now();

                // check next 7 days for the first matching day
                for i in 1..=7 {
                    let candidate = now + Duration::days(i);
                    if days.contains(&candidate.weekday()) {
                        let candidate_time = candidate
                            .with_hour(*hour as u32)
                            .and_then(|t| t.with_minute(*minute as u32))
                            .and_then(|t| t.with_second(0))
                            .unwrap();
                        return Some(candidate_time);
                    }
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
