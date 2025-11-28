/// Utility functions for date format handling in commands
use crate::application::services::timezone_service::TimezoneService;
use std::sync::Arc;

/// Get a human-readable description of a date format
pub fn get_date_format_description(format: &str) -> &'static str {
    match format {
        "DMY" => "DD-MM-YYYY (e.g., 27-11-2025)",
        "MDY" => "MM-DD-YYYY (e.g., 11-27-2025)",
        "YMD" => "YYYY-MM-DD (e.g., 2025-11-27)",
        _ => "YYYY-MM-DD (e.g., 2025-11-27)",
    }
}

/// Get inferred date format information for a timezone
pub fn get_inferred_date_format_info(
    timezone_service: &Arc<TimezoneService>,
    timezone_id: &str,
) -> (&'static str, &'static str) {
    let inferred_format = timezone_service
        .infer_date_format_from_timezone(timezone_id) // Usa el método público ahora
        .unwrap_or("YMD");

    let format_description = get_date_format_description(inferred_format);
    (inferred_format, format_description)
}

/// Get user's date format information for display
pub async fn get_user_date_format_info(
    timezone_service: &Arc<TimezoneService>,
    user_id: u64,
) -> String {
    match timezone_service.get_user_date_format(user_id).await {
        Ok(Some(format)) => {
            let format_desc = get_date_format_description(&format);
            format!("`{}` - {}", format, format_desc)
        }
        _ => "Automatically set based on your location".to_string(),
    }
}
