// src/infrastructure/repositories/sqlite_task_repository.rs
use crate::domain::repositories::TaskRepository;
use crate::domain::{NotificationMethod, Recurrence, Task};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, params};
use serde_json;
use std::sync::{Arc, Mutex};

pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    pub fn new(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tasks (
                id               INTEGER PRIMARY KEY,
                user_id          INTEGER NOT NULL,
                guild_id         INTEGER NOT NULL,
                title            TEXT NOT NULL,
                description      TEXT,
                scheduled_time   INTEGER,
                recurrence_type  TEXT,
                recurrence_data  TEXT,
                notification_method TEXT NOT NULL,
                channel_id       INTEGER,
                mention          TEXT
            );
            ",
        )
        .map_err(|e| e.to_string())?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // NOTE: helper to convert a rusqlite::Row -> Task; kept synchronous because it runs inside spawn_blocking
    fn row_to_task(row: &rusqlite::Row) -> Result<Task, String> {
        let id: i64 = row.get("id").map_err(|e| e.to_string())?;
        let user_id: i64 = row.get("user_id").map_err(|e| e.to_string())?;
        let guild_id: i64 = row.get("guild_id").map_err(|e| e.to_string())?;
        let title: String = row.get("title").map_err(|e| e.to_string())?;
        let description: Option<String> = row.get("description").map_err(|e| e.to_string())?;

        // datetime
        let ts: Option<i64> = row.get("scheduled_time").map_err(|e| e.to_string())?;
        let scheduled_time = ts.map(|t| Utc.timestamp_opt(t, 0).unwrap());

        // recurrence
        let recurrence_type: Option<String> =
            row.get("recurrence_type").map_err(|e| e.to_string())?;
        let recurrence_data: Option<String> =
            row.get("recurrence_data").map_err(|e| e.to_string())?;

        let recurrence = match (recurrence_type.as_deref(), recurrence_data) {
            (Some("weekly"), Some(json)) => {
                let d: crate::domain::WeeklyRecurrenceData =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                Some(Recurrence::Weekly {
                    days: d.days,
                    hour: d.hour,
                    minute: d.minute,
                })
            }
            (Some("every_x_days"), Some(json)) => {
                let d: crate::domain::EveryXDaysRecurrenceData =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                Some(Recurrence::EveryXDays {
                    interval: d.interval,
                    hour: d.hour,
                    minute: d.minute,
                })
            }
            _ => None,
        };

        // notification
        let notif: String = row.get("notification_method").map_err(|e| e.to_string())?;
        let notification_method = match notif.as_str() {
            "dm" => NotificationMethod::DM,
            "channel" => NotificationMethod::Channel,
            "both" => NotificationMethod::Both,
            _ => NotificationMethod::DM,
        };

        let channel_id: Option<i64> = row.get("channel_id").map_err(|e| e.to_string())?;
        let mention: Option<String> = row.get("mention").map_err(|e| e.to_string())?;

        Ok(Task::new(
            id as u64,
            user_id as u64,
            guild_id as u64,
            title,
            description,
            scheduled_time,
            recurrence,
            notification_method,
            channel_id.map(|v| v as u64),
            mention,
        ))
    }
}

#[async_trait]
impl TaskRepository for SqliteTaskRepository {
    async fn add_task(&self, mut task: Task) -> Result<u64, String> {
        let conn = self.conn.clone();
        
        tokio::task::spawn_blocking(move || -> Result<u64, String> {
                // compute next id using MAX(id) (note: we'll address autoincrement in step 2)
                let id_opt: Option<i64> = {
                    let conn_lock = conn.lock().unwrap();
                    conn_lock
                        .query_row("SELECT MAX(id) FROM tasks", [], |row| row.get(0))
                        .map_err(|e| e.to_string())?
                };
                let id = id_opt.unwrap_or(0) as u64 + 1;
                task.id = id;

                let scheduled_ts = task.scheduled_time.map(|dt| dt.timestamp());

                let (rec_type, rec_data) = match &task.recurrence {
                    Some(Recurrence::Weekly { days, hour, minute }) => {
                        let json = serde_json::to_string(&crate::domain::WeeklyRecurrenceData {
                            days: days.clone(),
                            hour: *hour,
                            minute: *minute,
                        })
                        .map_err(|e| e.to_string())?;
                        (Some("weekly".to_string()), Some(json))
                    }
                    Some(Recurrence::EveryXDays {
                        interval,
                        hour,
                        minute,
                    }) => {
                        let json = serde_json::to_string(&crate::domain::EveryXDaysRecurrenceData {
                            interval: *interval,
                            hour: *hour,
                            minute: *minute,
                        })
                        .map_err(|e| e.to_string())?;
                        (Some("every_x_days".to_string()), Some(json))
                    }
                    None => (None, None),
                };

                let notif = match task.notification_method {
                    NotificationMethod::DM => "dm",
                    NotificationMethod::Channel => "channel",
                    NotificationMethod::Both => "both",
                };

                let conn_lock = conn.lock().unwrap();
                conn_lock
                    .execute(
                        "INSERT INTO tasks (
                            id, user_id, guild_id, title, description, scheduled_time,
                            recurrence_type, recurrence_data,
                            notification_method, channel_id, mention
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![
                            id as i64,
                            task.user_id as i64,
                            task.guild_id as i64,
                            task.title,
                            task.description,
                            scheduled_ts,
                            rec_type,
                            rec_data,
                            notif,
                            task.channel_id.map(|v| v as i64),
                            task.mention
                        ],
                    )
                    .map_err(|e| e.to_string())?;

                Ok(id)
            })
            .await
            .map_err(|e| e.to_string())?
    }

    async fn edit_task(
        &self,
        task_id: u64,
        new_title: Option<String>,
        new_description: Option<String>,
        new_scheduled_time: Option<DateTime<Utc>>,
        new_recurrence: Option<Recurrence>,
        new_notification_method: Option<NotificationMethod>,
    ) -> Result<Task, String> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Task, String> {
                // obtain task
                let conn_lock = conn.lock().unwrap();

                let mut stmt = conn_lock
                    .prepare("SELECT * FROM tasks WHERE id = ?1")
                    .map_err(|e| e.to_string())?;

                let task: Task = stmt
                    .query_row(params![task_id as i64], |row| {
                        SqliteTaskRepository::row_to_task(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))
                    })
                    .map_err(|_| format!("Couldn't find task with ID {}", task_id))?;

                let mut updated = task.clone();

                // apply changes
                if let Some(t) = new_title {
                    if t.trim().is_empty() {
                        return Err("Task title cannot be empty".to_string());
                    }
                    updated.title = t;
                }
                if let Some(d) = new_description {
                    updated.description = Some(d);
                }
                if let Some(t) = new_scheduled_time {
                    updated.scheduled_time = Some(t);
                }
                if let Some(r) = new_recurrence {
                    updated.recurrence = Some(r);
                }
                if let Some(n) = new_notification_method {
                    updated.notification_method = n;
                }

                // prepare fields
                let scheduled_ts = updated.scheduled_time.map(|dt| dt.timestamp());

                let (rec_type, rec_data) = match &updated.recurrence {
                    Some(Recurrence::Weekly { days, hour, minute }) => {
                        let json = serde_json::to_string(&crate::domain::WeeklyRecurrenceData {
                            days: days.clone(),
                            hour: *hour,
                            minute: *minute,
                        })
                        .map_err(|e| e.to_string())?;
                        (Some("weekly".to_string()), Some(json))
                    }
                    Some(Recurrence::EveryXDays {
                        interval,
                        hour,
                        minute,
                    }) => {
                        let json = serde_json::to_string(&crate::domain::EveryXDaysRecurrenceData {
                            interval: *interval,
                            hour: *hour,
                            minute: *minute,
                        })
                        .map_err(|e| e.to_string())?;
                        (Some("every_x_days".to_string()), Some(json))
                    }
                    None => (None, None),
                };

                let notif = match updated.notification_method {
                    NotificationMethod::DM => "dm",
                    NotificationMethod::Channel => "channel",
                    NotificationMethod::Both => "both",
                };

                conn_lock
                    .execute(
                        "UPDATE tasks SET
                            user_id = ?2,
                            guild_id = ?3,
                            title = ?4,
                            description = ?5,
                            scheduled_time = ?6,
                            recurrence_type = ?7,
                            recurrence_data = ?8,
                            notification_method = ?9,
                            channel_id = ?10,
                            mention = ?11
                         WHERE id = ?1",
                        params![
                            task_id as i64,
                            updated.user_id as i64,
                            updated.guild_id as i64,
                            updated.title,
                            updated.description,
                            scheduled_ts,
                            rec_type,
                            rec_data,
                            notif,
                            updated.channel_id.map(|v| v as i64),
                            updated.mention
                        ],
                    )
                    .map_err(|e| e.to_string())?;

                Ok(updated)
            })
            .await
            .map_err(|e| e.to_string())?
    }

    async fn remove_task(&self, task_id: u64) -> bool {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> bool {
                let conn_lock = conn.lock().unwrap();
                conn_lock
                    .execute("DELETE FROM tasks WHERE id = ?1", params![task_id as i64])
                    .unwrap_or(0)
                    > 0
            })
            .await
            .unwrap_or(false)
    }

    async fn remove_all_by_user(&self, user_id: u64) -> usize {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> usize {
                let conn_lock = conn.lock().unwrap();
                conn_lock
                    .execute("DELETE FROM tasks WHERE user_id = ?1", params![user_id as i64])
                    .unwrap_or(0) as usize
            })
            .await
            .unwrap_or(0)
    }

    async fn list_tasks(&self) -> Vec<Task> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Vec<Task> {
                let conn_lock = conn.lock().unwrap();

                let mut stmt = match conn_lock.prepare("SELECT * FROM tasks") {
                    Ok(s) => s,
                    Err(_) => return Vec::new(),
                };

                let iter = match stmt.query_map([], |row| {
                    SqliteTaskRepository::row_to_task(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))
                }) {
                    Ok(it) => it,
                    Err(_) => return Vec::new(),
                };

                iter.filter_map(|r| r.ok()).collect()
            })
            .await
            .unwrap_or_else(|_| Vec::new())
    }

    /// Get total count of all tasks in the system (admin only)
    async fn get_total_task_count(&self) -> Result<u64, String> {
        let conn = self.conn.clone();
        
        let count: Result<u64, String> = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| e.to_string())?;
            
            let mut stmt = conn
                .prepare("SELECT COUNT(*) FROM tasks")
                .map_err(|e| e.to_string())?;
                
            let count: i64 = stmt
                .query_row([], |row| row.get(0))
                .map_err(|e| e.to_string())?;
                
            Ok(count as u64)
        })
        .await
        .map_err(|e| e.to_string())?;
        
        count
    }

    async fn update_task_time(&self, task_id: u64, new_time: DateTime<Utc>) -> Result<(), String> {
        let conn = self.conn.clone();
        let ts = new_time.timestamp();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
                let conn_lock = conn.lock().unwrap();
                conn_lock
                    .execute(
                        "UPDATE tasks SET scheduled_time = ?2 WHERE id = ?1",
                        params![task_id as i64, ts],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .await
            .map_err(|e| e.to_string())?
    }
}
