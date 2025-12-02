//! Uses spawn_blocking to avoid blocking the async runtime.
//! Stores scheduled tasks in `scheduled_tasks` table (see schema.sql).

use crate::domain::entities::scheduled_task::ScheduledTask;
use crate::domain::repositories::task_scheduler_repository::{
    SchedulerError, TaskSchedulerRepository,
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, Row, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Simple helper to map NotificationMethod <-> string
fn notification_method_to_str(
    n: &crate::domain::entities::task::NotificationMethod,
) -> &'static str {
    match n {
        crate::domain::entities::task::NotificationMethod::DM => "dm",
        crate::domain::entities::task::NotificationMethod::Channel => "channel",
        crate::domain::entities::task::NotificationMethod::Both => "both",
    }
}

fn notification_method_from_str(s: &str) -> crate::domain::entities::task::NotificationMethod {
    match s {
        "channel" => crate::domain::entities::task::NotificationMethod::Channel,
        "both" => crate::domain::entities::task::NotificationMethod::Both,
        _ => crate::domain::entities::task::NotificationMethod::DM,
    }
}

/// The repository holds a shared Connection guarded by Mutex so it is safe to use from multiple threads.
#[derive(Debug, Clone)]
pub struct SqliteSchedulerRepository {
    conn: Arc<Mutex<Connection>>,
    // Channel to notify scheduler when new tasks are added
    wakeup_sender: broadcast::Sender<()>,
}

impl SqliteSchedulerRepository {
    /// Create or open DB at `db_path` and ensure scheduled_tasks table exists.
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, SchedulerError> {
        let conn = Connection::open(db_path.as_ref())
            .map_err(|e| SchedulerError::StorageError(format!("Failed to open DB: {}", e)))?;

        // Ensure table exists (synchronous init)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS scheduled_tasks (
                task_id         INTEGER PRIMARY KEY,
                scheduled_time  INTEGER NOT NULL,
                user_id         INTEGER NOT NULL,
                guild_id        INTEGER NOT NULL,
                title           TEXT NOT NULL,
                notification_method TEXT NOT NULL,
                is_recurring    INTEGER NOT NULL DEFAULT 0,
                is_deleted      INTEGER NOT NULL DEFAULT 0,
                mention         TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_time ON scheduled_tasks (is_deleted, scheduled_time);
            "#,
        )
        .map_err(|e| SchedulerError::StorageError(format!("Failed to initialize scheduler table: {}", e)))?;

        let (wakeup_sender, _) = broadcast::channel(1);
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            wakeup_sender,
        })
    }

    /// Get a receiver for wake-up notifications (for scheduler to react to new tasks)
    pub fn subscribe_wakeup(&self) -> broadcast::Receiver<()> {
        self.wakeup_sender.subscribe()
    }

    /// Helper: build a ScheduledTask from a rusqlite::Row
    fn row_to_scheduled_task(row: &Row) -> Result<ScheduledTask, SchedulerError> {
        let task_id: i64 = row
            .get(0)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let scheduled_time_ts: i64 = row
            .get(1)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let user_id: i64 = row
            .get(2)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let guild_id: i64 = row
            .get(3)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let title: String = row
            .get(4)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let notification_method_str: String = row
            .get(5)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let is_recurring_i: i64 = row
            .get(6)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let is_deleted_i: i64 = row
            .get(7)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
        let mention: Option<String> = row
            .get(8)
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;

        let scheduled_time = Utc
            .timestamp_opt(scheduled_time_ts, 0)
            .single()
            .ok_or_else(|| SchedulerError::StorageError("Invalid timestamp in DB".to_string()))?;

        Ok(ScheduledTask {
            task_id: task_id as u64,
            scheduled_time,
            user_id: user_id as u64,
            guild_id: guild_id as u64,
            title,
            notification_method: notification_method_from_str(&notification_method_str),
            is_recurring: is_recurring_i != 0,
            is_deleted: is_deleted_i != 0,
            mention,
        })
    }
}

#[async_trait]
impl TaskSchedulerRepository for SqliteSchedulerRepository {
    /// Insert or replace a scheduled task (upsert). Runs in spawn_blocking and is non-blocking for the async runtime.
    async fn add_scheduled_task(&self, task: ScheduledTask) -> Result<(), SchedulerError> {
        let conn = self.conn.clone();

        let task_clone = task.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn_lock = conn.lock()
                .map_err(|e| SchedulerError::StorageError(format!("Lock poisoned: {}", e)))?;
            let tx = conn_lock
                .transaction()
                .map_err(|e| SchedulerError::StorageError(e.to_string()))?;

            tx.execute(
                r#"
                INSERT INTO scheduled_tasks (
                    task_id, scheduled_time, user_id, guild_id, title,
                    notification_method, is_recurring, is_deleted, mention
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(task_id) DO UPDATE SET
                    scheduled_time = excluded.scheduled_time,
                    user_id = excluded.user_id,
                    guild_id = excluded.guild_id,
                    title = excluded.title,
                    notification_method = excluded.notification_method,
                    is_recurring = excluded.is_recurring,
                    is_deleted = 0,
                    mention = excluded.mention
                "#,
                params![
                    task_clone.task_id as i64,
                    task_clone.scheduled_time.timestamp(),
                    task_clone.user_id as i64,
                    task_clone.guild_id as i64,
                    task_clone.title,
                    notification_method_to_str(&task_clone.notification_method),
                    if task_clone.is_recurring { 1 } else { 0 },
                    0i64, // clear is_deleted on upsert
                    task_clone.mention
                ],
            )
            .map_err(|e| SchedulerError::StorageError(e.to_string()))?;

            tx.commit()
                .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|_| SchedulerError::StorageError("Task join error".into()))??;
        
        // Send wake-up signal to notify scheduler of new task
        let _ = self.wakeup_sender.send(());
        
        Ok(())
    }

    /// Return next (non-deleted) scheduled task without removing it, ordered by scheduled_time ASC.
    async fn peek_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn_lock = conn.lock()
                .map_err(|e| SchedulerError::StorageError(format!("Lock poisoned: {}", e)))?;
            let mut stmt = conn_lock.prepare(
                "SELECT task_id, scheduled_time, user_id, guild_id, title, notification_method, is_recurring, is_deleted, mention
                 FROM scheduled_tasks WHERE is_deleted = 0 ORDER BY scheduled_time ASC LIMIT 1",
            ).map_err(|e| SchedulerError::StorageError(e.to_string()))?;

            let mut rows = stmt.query([]).map_err(|e| SchedulerError::StorageError(e.to_string()))?;
            match rows.next().map_err(|e| SchedulerError::StorageError(e.to_string()))? {
                Some(row) => SqliteSchedulerRepository::row_to_scheduled_task(&row).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|_| SchedulerError::StorageError("Task join error".into()))?
    }

    /// Remove and return the next pending task (non-deleted). This deletes the row from DB.
    async fn pop_next_task(&self) -> Result<Option<ScheduledTask>, SchedulerError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn_lock = conn.lock()
                .map_err(|e| SchedulerError::StorageError(format!("Lock poisoned: {}", e)))?;
            let tx = conn_lock.transaction().map_err(|e| SchedulerError::StorageError(e.to_string()))?;

            // Select next non-deleted
            let task_opt = {
                let mut stmt = tx.prepare(
                    "SELECT task_id, scheduled_time, user_id, guild_id, title, notification_method, is_recurring, is_deleted, mention
                     FROM scheduled_tasks WHERE is_deleted = 0 ORDER BY scheduled_time ASC LIMIT 1",
                ).map_err(|e| SchedulerError::StorageError(e.to_string()))?;

                let mut rows = stmt.query([]).map_err(|e| SchedulerError::StorageError(e.to_string()))?;
                if let Some(row) = rows.next().map_err(|e| SchedulerError::StorageError(e.to_string()))? {
                    Some(SqliteSchedulerRepository::row_to_scheduled_task(&row)?)
                } else {
                    None
                }
            };

            if let Some(task) = task_opt {
                // Delete selected task (pop)
                tx.execute(
                    "DELETE FROM scheduled_tasks WHERE task_id = ?1",
                    params![task.task_id as i64],
                )
                .map_err(|e| SchedulerError::StorageError(e.to_string()))?;

                tx.commit().map_err(|e| SchedulerError::StorageError(e.to_string()))?;
                Ok(Some(task))
            } else {
                // nothing to pop
                Ok(None)
            }
        })
        .await
        .map_err(|_| SchedulerError::StorageError("Task join error".into()))?
    }

    /// Soft-delete (mark is_deleted = 1) a scheduled task by task_id.
    async fn remove_task(&self, task_id: u64) -> Result<(), SchedulerError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn_lock = conn.lock()
                .map_err(|e| SchedulerError::StorageError(format!("Lock poisoned: {}", e)))?;
            let affected = conn_lock
                .execute(
                    "UPDATE scheduled_tasks SET is_deleted = 1 WHERE task_id = ?1 AND is_deleted = 0",
                    params![task_id as i64],
                )
                .map_err(|e| SchedulerError::StorageError(e.to_string()))?;

            if affected == 0 {
                // If we didn't update any row, consider TaskNotFound
                return Err(SchedulerError::TaskNotFound);
            }
            Ok(())
        })
        .await
        .map_err(|_| SchedulerError::StorageError("Task join error".into()))?
    }

    /// Check if there are any pending (non-deleted) tasks.
    async fn has_pending_tasks(&self) -> Result<bool, SchedulerError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn_lock = conn.lock()
                .map_err(|e| SchedulerError::StorageError(format!("Lock poisoned: {}", e)))?;
            let count: i64 = conn_lock
                .query_row(
                    "SELECT COUNT(1) FROM scheduled_tasks WHERE is_deleted = 0",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| SchedulerError::StorageError(e.to_string()))?;
            Ok(count > 0)
        })
        .await
        .map_err(|_| SchedulerError::StorageError("Task join error".into()))?
    }
}
