use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::task;

use crate::domain::repositories::config_repository::ConfigRepository;

pub struct SqliteConfigRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteConfigRepository {
    /// Opens the SQLite DB and initializes the schema if needed.
    pub async fn new(db_path: &str) -> Result<Self, String> {
        let db_path_owned = db_path.to_string();

        // Opening SQLite is blocking → run in blocking thread.
        let conn = task::spawn_blocking(move || {
            Connection::open(db_path_owned)
                .map_err(|e| format!("Failed to open SQLite DB: {}", e))
        })
        .await
        .map_err(|e| format!("Join error: {}", e))??;

        let repo = Self {
            connection: Arc::new(Mutex::new(conn)),
        };

        repo.initialize_schema().await?;
        Ok(repo)
    }

    /// Creates the config table.
    async fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.connection.clone();

        task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS config (
                    guild_id    INTEGER PRIMARY KEY,
                    channel_id  INTEGER NOT NULL
                );
                "#,
                [],
            )
            .map_err(|e| format!("Failed to create config table: {}", e))?;

            Ok(())
        })
        .await
        .map_err(|e| format!("Join error: {}", e))?
    }
}

#[async_trait]
impl ConfigRepository for SqliteConfigRepository {
    /// Sets or updates a notification channel for a guild.
    /// This runs inside spawn_blocking to avoid blocking the async executor.
    async fn set_notification_channel(&self, guild_id: u64, channel_id: u64) {
        let conn = self.connection.clone();

        let _ = task::spawn_blocking(move || {
            let conn = match conn.lock() {
                Ok(c) => c,
                Err(_) => return, // poisoned mutex → silently fail
            };

            let _ = conn.execute(
                r#"
                INSERT INTO config (guild_id, channel_id)
                VALUES (?1, ?2)
                ON CONFLICT(guild_id)
                DO UPDATE SET channel_id = excluded.channel_id
                "#,
                params![guild_id as i64, channel_id as i64],
            );
        })
        .await;
    }

    /// Reads the configured notification channel for a guild.
    async fn get_notification_channel(&self, guild_id: u64) -> Option<u64> {
        let conn = self.connection.clone();

        task::spawn_blocking(move || {
            let conn = conn.lock().ok()?;

            let mut stmt = conn.prepare("SELECT channel_id FROM config WHERE guild_id = ?1").ok()?;

            let result = stmt
                .query_row(params![guild_id as i64], |row| row.get::<_, i64>(0))
                .ok()?;

            Some(result as u64)
        })
        .await
        .ok()?
    }
}

impl Clone for SqliteConfigRepository {
    fn clone(&self) -> Self {
        Self {
            connection: Arc::clone(&self.connection),
        }
    }
}

impl std::fmt::Debug for SqliteConfigRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteConfigRepository").finish()
    }
}
