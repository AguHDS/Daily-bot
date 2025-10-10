use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::domain::repositories::config_repository::ConfigRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub notification_channels: HashMap<u64, u64>,
}

pub struct JsonConfigRepository {
    file_path: PathBuf,
    data: RwLock<ConfigData>,
}

impl JsonConfigRepository {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        let file_path = file_path.into();
        let data = Self::load_data(&file_path).unwrap_or_else(|_| ConfigData {
            notification_channels: HashMap::new(),
        });

        Self {
            file_path,
            data: RwLock::new(data),
        }
    }

    fn load_data(file_path: &PathBuf) -> Result<ConfigData, Box<dyn std::error::Error>> {
        if !file_path.exists() {
            return Ok(ConfigData {
                notification_channels: HashMap::new(),
            });
        }

        let content = fs::read_to_string(file_path)?;
        let data: ConfigData = serde_json::from_str(&content)?;
        Ok(data)
    }

    fn save_data_with_data(&self, data: ConfigData) {
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                // Create dir if don't exists
                if let Some(parent) = self.file_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        eprintln!("Failed to create directory: {}", e);
                        return;
                    }
                }

                if let Err(e) = fs::write(&self.file_path, json) {
                    eprintln!("Failed to write file: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize data: {}", e);
            }
        }
    }
}

impl ConfigRepository for JsonConfigRepository {
    fn set_notification_channel(&self, guild_id: u64, channel_id: u64) {
        let data_to_save = {
            if let Ok(mut data) = self.data.write() {
                data.notification_channels.insert(guild_id, channel_id);

                data.clone()
            } else {
                return;
            }
        };

        self.save_data_with_data(data_to_save);
    }

    fn get_notification_channel(&self, guild_id: u64) -> Option<u64> {
        let data = self.data.read().ok()?;
        data.notification_channels.get(&guild_id).copied()
    }
}

impl Clone for JsonConfigRepository {
    fn clone(&self) -> Self {
        let data = self.data.read().unwrap().clone();
        Self {
            file_path: self.file_path.clone(),
            data: RwLock::new(data),
        }
    }
}

impl std::fmt::Debug for JsonConfigRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonConfigRepository")
            .field("file_path", &self.file_path)
            .finish()
    }
}
