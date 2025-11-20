use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

/// Primary manager for SQLite database operations; provides async-friendly access to synchronous rusqlite connections using tokio's spawn_blocking.
#[derive(Clone)]
pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    /// Create a new instance of the DatabaseManager; opens the SQLite database and configures it for better performance.
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(db_path)?;

        // Enable foreign keys and WAL mode for better concurrency and performance.
        connection.execute_batch(
            "PRAGMA foreign_keys = ON; 
             PRAGMA journal_mode = WAL; 
             PRAGMA synchronous = NORMAL;",
        )?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Execute a blocking database operation in a tokio-aware manner; moves the operation to a blocking thread pool to avoid blocking the async runtime.
    pub async fn execute_blocking<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn = connection.lock().unwrap();
            operation(&conn)
        })
        .await
        .context("Failed to execute blocking database operation - task join error")?
        .context("Database operation failed")
    }

    /// Initialize the database by creating all the tables; reads and executes schema.sql to set up the database structure.
    pub async fn initialize_database(&self) -> Result<()> {
        let schema = include_str!("schema.sql");

        self.execute_blocking(move |connection| {
            let statements: Vec<&str> = schema.split(';').collect();
            for (i, statement) in statements.iter().enumerate() {
                let trimmed = statement.trim();

                // Skip empty statements and comments.
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    let preview = if trimmed.len() > 50 {
                        format!("{}...", &trimmed[..50])
                    } else {
                        trimmed.to_string()
                    };
                    println!("📝 Executing statement {}: {}", i + 1, preview);

                    match connection.execute(trimmed, []) {
                        Ok(rows) => println!(
                            "✅ Statement {} executed successfully (affected {} rows)",
                            i + 1,
                            rows
                        ),
                        Err(e) => {
                            println!("❌ ERROR in statement {}: {}", i + 1, e);
                            println!("   Full statement: {}", trimmed);
                            return Err(e);
                        }
                    }
                }
            }
            Ok(())
        })
        .await
    }
}

/// Custom result type for database operations using anyhow for error handling.
#[allow(dead_code)]
pub type DatabaseResult<T> = Result<T>;
