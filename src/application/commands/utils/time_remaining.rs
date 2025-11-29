use chrono::{DateTime, Duration, Utc};

/// Calculates and formats the time remaining until a target UTC datetime
/// Returns a human-readable string like "2hs y 16 mins", "23 horas", "5 días y 2 horas"
pub fn format_time_remaining(target: DateTime<Utc>) -> String {
    let now = Utc::now();

    if target <= now {
        return "Due now".to_string();
    }

    let duration = target - now;

    format_duration(duration)
}

/// Formats a duration into a human-readable string
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    match (days, hours, minutes) {
        (0, 0, 0) => "Less than 1 minute".to_string(),
        (0, 0, mins) => format!("{} min{}", mins, if mins != 1 { "s" } else { "" }),
        (0, hrs, 0) => format!("{} hora{}", hrs, if hrs != 1 { "s" } else { "" }),
        (0, hrs, mins) => format!(
            "{}hs y {} min{}",
            hrs,
            mins,
            if mins != 1 { "s" } else { "" }
        ),
        (days, 0, 0) => format!("{} día{}", days, if days != 1 { "s" } else { "" }),
        (days, hrs, 0) => format!(
            "{} día{} y {} hora{}",
            days,
            if days != 1 { "s" } else { "" },
            hrs,
            if hrs != 1 { "s" } else { "" }
        ),
        (days, hrs, mins) => format!(
            "{} día{}, {}hs y {} min{}",
            days,
            if days != 1 { "s" } else { "" },
            hrs,
            mins,
            if mins != 1 { "s" } else { "" }
        ),
    }
}
