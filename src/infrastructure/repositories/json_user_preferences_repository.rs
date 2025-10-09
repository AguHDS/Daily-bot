use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::sync::RwLock as AsyncRwLock;

use crate::domain::entities::user_preferences::UserPreferences;
use crate::domain::repositories::user_preferences_repository::{
    RepositoryError, UserPreferencesRepository,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPreferencesData {
    users: HashMap<u64, UserPreferences>,
}

pub struct JsonUserPreferencesRepository {
    file_path: PathBuf,
    data: AsyncRwLock<HashMap<u64, UserPreferences>>,
}

impl JsonUserPreferencesRepository {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        let file_path = file_path.into();

        let data = Self::load_data(&file_path).unwrap_or_default();
        println!("💾 [DEBUG] Datos cargados: {} usuarios", data.len());

        Self {
            file_path,
            data: AsyncRwLock::new(data),
        }
    }

    fn load_data(file_path: &PathBuf) -> Result<HashMap<u64, UserPreferences>, RepositoryError> {
        if !file_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(file_path)
            .map_err(|e| RepositoryError::StorageError(format!("Failed to read file: {}", e)))?;

        let data: UserPreferencesData = serde_json::from_str(&content)
            .map_err(|e| RepositoryError::StorageError(format!("Failed to parse JSON: {}", e)))?;

        Ok(data.users)
    }

    async fn save_data(&self) -> Result<(), RepositoryError> {
        let data = {
            let lock = self.data.read().await;
            lock.clone()
        };

        let user_prefs_data = UserPreferencesData { users: data };

        let json = serde_json::to_string_pretty(&user_prefs_data).map_err(|e| {
            RepositoryError::StorageError(format!("Failed to serialize JSON: {}", e))
        })?;

        // create directory
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RepositoryError::StorageError(format!("Failed to create directory: {}", e))
            })?;
        }

        // write file
        fs::write(&self.file_path, &json)
            .map_err(|e| RepositoryError::StorageError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl UserPreferencesRepository for JsonUserPreferencesRepository {
    async fn get(&self, user_id: u64) -> Result<Option<UserPreferences>, RepositoryError> {
        let data = self.data.read().await;
        let result = data.get(&user_id).cloned();
        Ok(result)
    }

    async fn save(&self, preferences: &UserPreferences) -> Result<(), RepositoryError> {
        let mut data = self.data.write().await;

        if !preferences.is_valid() {
            return Err(RepositoryError::InvalidData(
                "Invalid user preferences".to_string(),
            ));
        }

        data.insert(preferences.user_id, preferences.clone());

        drop(data);

        match self.save_data().await {
            Ok(()) => {
                println!("Save successful");
                Ok(())
            }
            Err(e) => {
                println!("Error saving: {:?}", e);
                Err(e)
            }
        }
    }

    async fn delete(&self, user_id: u64) -> Result<(), RepositoryError> {
        let mut data = self.data.write().await;

        if data.remove(&user_id).is_none() {
            return Err(RepositoryError::NotFound);
        }

        self.save_data().await
    }
}

impl std::fmt::Debug for JsonUserPreferencesRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonUserPreferencesRepository")
            .field("file_path", &self.file_path)
            .finish()
    }
}
