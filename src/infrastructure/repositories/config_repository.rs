use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::domain::repositories::ConfigRepository;

/// In-memory implementation of ConfigRepository.
/// Useful for testing or simple setups without a real database.
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct InMemoryConfigRepository {
    guild_channels: Arc<RwLock<HashMap<u64, u64>>>,
}

#[allow(dead_code)]
impl InMemoryConfigRepository {
    pub fn new() -> Self {
        Self {
            guild_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ConfigRepository for InMemoryConfigRepository {
    // ← Usa el trait del domain
    fn set_notification_channel(&self, guild_id: u64, channel_id: u64) {
        if let Ok(mut map) = self.guild_channels.write() {
            map.insert(guild_id, channel_id);
        }
    }

    fn get_notification_channel(&self, guild_id: u64) -> Option<u64> {
        self.guild_channels.read().ok()?.get(&guild_id).copied()
    }
}
