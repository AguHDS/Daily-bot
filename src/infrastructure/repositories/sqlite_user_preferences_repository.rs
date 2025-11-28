use async_trait::async_trait;
use chrono::TimeZone;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::domain::entities::user_preferences::UserPreferences;
use crate::domain::repositories::user_preferences_repository::{
    RepositoryError, UserPreferencesRepository,
};

pub struct SqliteUserPreferencesRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteUserPreferencesRepository {
    pub fn new(db_path: impl Into<PathBuf>) -> Result<Self, RepositoryError> {
        let path = db_path.into();

        let conn = Connection::open(path).map_err(|e| {
            RepositoryError::StorageError(format!("Failed to open SQLite DB: {}", e))
        })?;

        let repo = Self {
            connection: Arc::new(Mutex::new(conn)),
        };

        repo.initialize_schema()?;
        Ok(repo)
    }

    fn initialize_schema(&self) -> Result<(), RepositoryError> {
        let conn = self.connection.lock().unwrap();

        // Add date_format column with ALTER TABLE to maintain backward compatibility
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS user_preferences (
                user_id        INTEGER PRIMARY KEY,
                timezone       TEXT NOT NULL,
                date_format    TEXT, -- NULL for backward compatibility
                created_at     INTEGER NOT NULL,
                updated_at     INTEGER NOT NULL
            );
            "#,
            [],
        )
        .map_err(|e| RepositoryError::StorageError(format!("Failed to create table: {}", e)))?;

        // Try to add the date_format column if it doesn't exist (for existing databases)
        let _ = conn.execute(
            "ALTER TABLE user_preferences ADD COLUMN date_format TEXT;",
            [],
        );

        Ok(())
    }
}

#[async_trait]
impl UserPreferencesRepository for SqliteUserPreferencesRepository {
    async fn get(&self, user_id: u64) -> Result<Option<UserPreferences>, RepositoryError> {
        let conn = self.connection.clone();

        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            let mut stmt = conn.prepare(
                "SELECT user_id, timezone, date_format, created_at, updated_at
                 FROM user_preferences WHERE user_id = ?1",
            )?;

            let row = stmt.query_row(params![user_id as i64], |row| {
                let user_id_val = row.get::<_, i64>(0)? as u64;
                let timezone_val = row.get::<_, String>(1)?;
                let date_format_val = row.get::<_, Option<String>>(2)?;
                let created_at_val = row.get::<_, i64>(3)?;
                let updated_at_val = row.get::<_, i64>(4)?;

                Ok(UserPreferences {
                    user_id: user_id_val,
                    timezone: timezone_val,
                    date_format: date_format_val,
                    created_at: chrono::Utc.timestamp_opt(created_at_val, 0).unwrap(),
                    updated_at: chrono::Utc.timestamp_opt(updated_at_val, 0).unwrap(),
                })
            });

            match row {
                Ok(pref) => Ok(Some(pref)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|_| RepositoryError::StorageError("Task join error".into()))?;

        result.map_err(|e| RepositoryError::StorageError(e.to_string()))
    }

    async fn save(&self, preferences: &UserPreferences) -> Result<(), RepositoryError> {
        if !preferences.is_valid() {
            return Err(RepositoryError::InvalidData(
                "Invalid user preferences".into(),
            ));
        }

        let conn = self.connection.clone();
        let prefs = preferences.clone();

        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            conn.execute(
                r#"
                INSERT INTO user_preferences (user_id, timezone, date_format, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(user_id) DO UPDATE SET
                    timezone = excluded.timezone,
                    date_format = excluded.date_format,
                    updated_at = excluded.updated_at;
                "#,
                params![
                    prefs.user_id as i64,
                    prefs.timezone,
                    prefs.date_format,
                    prefs.created_at.timestamp(),
                    prefs.updated_at.timestamp()
                ],
            )?;

            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(|_| RepositoryError::StorageError("Task join error".into()))?;

        result.map_err(|e| RepositoryError::StorageError(e.to_string()))
    }

    async fn delete(&self, user_id: u64) -> Result<(), RepositoryError> {
        let conn = self.connection.clone();

        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            let affected = conn.execute(
                "DELETE FROM user_preferences WHERE user_id = ?1",
                params![user_id as i64],
            )?;

            if affected == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }

            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(|_| RepositoryError::StorageError("Task join error".into()))?;

        result.map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => RepositoryError::NotFound,
            other => RepositoryError::StorageError(other.to_string()),
        })
    }
}

impl std::fmt::Debug for SqliteUserPreferencesRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteUserPreferencesRepository").finish()
    }
}
