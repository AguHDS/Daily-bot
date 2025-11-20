use async_trait::async_trait;
use std::fmt::{Debug, Display};

use crate::domain::entities::user_preferences::UserPreferences;

#[allow(dead_code)]
#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    AlreadyExists,
    InvalidData(String),
    StorageError(String),
}

impl Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound => write!(f, "Resource not found"),
            RepositoryError::AlreadyExists => write!(f, "Resource already exists"),
            RepositoryError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            RepositoryError::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[async_trait]
pub trait UserPreferencesRepository: Send + Sync + Debug {
    /// Obtain user's preferences by user ID
    async fn get(&self, user_id: u64) -> Result<Option<UserPreferences>>;

    /// Save or update user's preferences
    async fn save(&self, preferences: &UserPreferences) -> Result<()>;

    /// Delete user's preferences by user ID
    #[allow(dead_code)]
    async fn delete(&self, user_id: u64) -> Result<()>;
}
