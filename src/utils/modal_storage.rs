use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Metadata stored temporarily for add_task modal processing
/// Allows multiple mentions in a single modal submission
#[derive(Clone, Debug)]
pub struct TaskModalMetadata {
    pub task_type: String,
    pub notification_method: String,
    pub channel_id: Option<u64>,
    pub mention: Option<String>,
    pub created_at: Instant,
}

impl TaskModalMetadata {
    pub fn new(
        task_type: String,
        notification_method: String,
        channel_id: Option<u64>,
        mention: Option<String>,
    ) -> Self {
        Self {
            task_type,
            notification_method,
            channel_id,
            mention,
            created_at: Instant::now(),
        }
    }
}

/// Temporary storage for modal metadata with automatic expiration
/// Prevents memory leaks by automatically cleaning up stale entries
#[derive(Clone)]
pub struct ModalStorage {
    storage: Arc<Mutex<HashMap<String, TaskModalMetadata>>>,
    ttl: Duration,
}

impl ModalStorage {
    /// Create a new ModalStorage with a Time-To-Live (TTL) for entries
    pub fn new(ttl: Duration) -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Store metadata with a unique ID
    pub async fn store(&self, id: String, metadata: TaskModalMetadata) {
        let mut storage = self.storage.lock().await;
        storage.insert(id, metadata);
    }

    /// Retrieve metadata by ID and remove it from storage
    /// Returns None if the ID doesn't exist or has expired
    pub async fn retrieve(&self, id: &str) -> Option<TaskModalMetadata> {
        let mut storage = self.storage.lock().await;
        
        if let Some(metadata) = storage.get(id) {
            // Check if expired
            if metadata.created_at.elapsed() > self.ttl {
                storage.remove(id);
                return None;
            }
            
            // Remove and return (one-time use)
            storage.remove(id)
        } else {
            None
        }
    }

    /// Clean up expired entries (can be called periodically)
    pub async fn cleanup_expired(&self) {
        let mut storage = self.storage.lock().await;
        storage.retain(|_, metadata| metadata.created_at.elapsed() <= self.ttl);
    }

    /// Get the number of stored entries (for debugging/monitoring)
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        let storage = self.storage.lock().await;
        storage.len()
    }
}

/// Generate a unique short ID for modal custom_id
/// Format: "add_task_modal_{timestamp}_{random}"
pub fn generate_modal_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    let random = rand::random::<u32>();
    
    format!("add_task_modal_{}_{}", timestamp, random)
}

