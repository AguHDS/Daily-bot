use async_trait::async_trait;
use std::fmt::Debug;

use crate::domain::entities::user_preferences::UserPreferences;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    AlreadyExists,
    InvalidData(String),
    StorageError(String),
}

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[async_trait]
pub trait UserPreferencesRepository: Send + Sync + Debug {
    /// Obtain user's preferences by user ID
    async fn get(&self, user_id: u64) -> Result<Option<UserPreferences>>;

    /// Save or update user's preferences
    async fn save(&self, preferences: &UserPreferences) -> Result<()>;

    /// Delete user's preferences by user ID
    async fn delete(&self, user_id: u64) -> Result<()>;

    /// Check if a preference for an user already exists
    async fn exists(&self, user_id: u64) -> Result<bool> {
        match self.get(user_id).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
