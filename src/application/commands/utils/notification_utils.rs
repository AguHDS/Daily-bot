use crate::domain::{NotificationMethod};
/// Convert NotificationMethod to &str
pub fn notification_method_as_str(method: &NotificationMethod) -> &str {
    match method {
        NotificationMethod::DM => "DM",
        NotificationMethod::Channel => "Channel",
        NotificationMethod::Both => "Both",
    }
}

/// Parse notification_method from string
pub fn parse_notification_method(s: &str) -> NotificationMethod {
    match s {
        "DM" => NotificationMethod::DM,
        "Channel" => NotificationMethod::Channel,
        "Both" => NotificationMethod::Both,
        _ => NotificationMethod::DM,
    }
}
