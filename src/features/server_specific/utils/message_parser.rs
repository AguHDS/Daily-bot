use tracing::error;

/// Extracts username from kick poll message
pub fn extract_username_from_kick_message(message: &str) -> Option<String> {
    let prefix = "Puedo kickear a ";
    let suffix = "?";

    if let Some(start) = message.find(prefix) {
        let start_idx = start + prefix.len();
        if let Some(end) = message.find(suffix) {
            if end > start_idx {
                return Some(message[start_idx..end].to_string());
            }
        }
    }

    error!("Failed to extract username from kick message: {}", message);
    None
}
