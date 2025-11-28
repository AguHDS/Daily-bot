use super::geo_mapping_service::GeoMappingService;
use crate::domain::entities::task::Recurrence;
use crate::domain::entities::user_preferences::UserPreferences;
use crate::domain::repositories::user_preferences_repository::{
    RepositoryError, UserPreferencesRepository,
};
use crate::domain::value_objects::weekday_format::WeekdayFormat;
use crate::infrastructure::timezone::timezone_manager::{TimezoneInfo, TimezoneManager};
use chrono::{DateTime, FixedOffset, LocalResult, TimeZone, Timelike, Utc, Weekday};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug)]
pub enum TimezoneError {
    RepositoryError(RepositoryError),
    InvalidTimezone(String),
    TimeConversionError(String),
    NotFound,
}

pub type Result<T> = std::result::Result<T, TimezoneError>;

pub struct TimezoneService {
    user_prefs_repo: Arc<dyn UserPreferencesRepository>,
    timezone_manager: Arc<TimezoneManager>,
    geo_mapping_service: GeoMappingService,
}

impl TimezoneService {
    pub fn new(
        user_prefs_repo: Arc<dyn UserPreferencesRepository>,
        timezone_manager: Arc<TimezoneManager>,
    ) -> Self {
        Self {
            user_prefs_repo,
            timezone_manager,
            geo_mapping_service: GeoMappingService::new(),
        }
    }

    /// Set the time zone for a user and infer date format automatically
    pub async fn set_user_timezone(&self, user_id: u64, timezone_str: &str) -> Result<()> {
        if !self.timezone_manager.validate_timezone(timezone_str) {
            return Err(TimezoneError::InvalidTimezone(format!(
                "Invalid timezone: {timezone_str}"
            )));
        }

        // Infer date format from timezone
        let date_format = self
            .geo_mapping_service
            .infer_date_format_from_timezone(timezone_str);

        let preferences = match self.user_prefs_repo.get(user_id).await {
            Ok(Some(mut prefs)) => {
                prefs.update_timezone_and_format(
                    timezone_str.to_string(),
                    date_format.map(|s| s.to_string()),
                );
                prefs
            }
            Ok(None) => UserPreferences::new_with_format(
                user_id,
                timezone_str.to_string(),
                date_format.map(|s| s.to_string()),
            ),
            Err(e) => return Err(TimezoneError::RepositoryError(e)),
        };

        self.user_prefs_repo
            .save(&preferences)
            .await
            .map_err(TimezoneError::RepositoryError)
    }

    /// Gets the time zone of a user
    pub async fn get_user_timezone(&self, user_id: u64) -> Result<Option<String>> {
        match self.user_prefs_repo.get(user_id).await {
            Ok(Some(prefs)) => Ok(Some(prefs.timezone)),
            Ok(None) => Ok(None),
            Err(e) => Err(TimezoneError::RepositoryError(e)),
        }
    }

    /// Gets the user's date format preference
    pub async fn get_user_date_format(&self, user_id: u64) -> Result<Option<String>> {
        match self.user_prefs_repo.get(user_id).await {
            Ok(Some(prefs)) => Ok(prefs.date_format),
            Ok(None) => Ok(None),
            Err(e) => Err(TimezoneError::RepositoryError(e)),
        }
    }

    /// Gets the user's complete preferences
    pub async fn get_user_preferences(&self, user_id: u64) -> Result<Option<UserPreferences>> {
        self.user_prefs_repo
            .get(user_id)
            .await
            .map_err(TimezoneError::RepositoryError)
    }

    /// Gets the date format placeholder for a user (for UI display)
    pub async fn get_user_date_format_placeholder(&self, user_id: u64) -> Result<&'static str> {
        match self.user_prefs_repo.get(user_id).await {
            Ok(Some(prefs)) => Ok(prefs.get_date_format_placeholder()),
            Ok(None) => Ok("YYYY-MM-DD"), // Default placeholder
            Err(e) => Err(TimezoneError::RepositoryError(e)),
        }
    }

    /// Convert a local date, time to UTC using a specific timezone
    pub fn parse_to_utc_with_timezone(
        &self,
        local_datetime: &str,
        timezone: &str,
    ) -> Result<DateTime<Utc>> {
        let naive_datetime =
            chrono::NaiveDateTime::parse_from_str(local_datetime, "%Y-%m-%d %H:%M").map_err(
                |e| {
                    TimezoneError::TimeConversionError(format!(
                        "Invalid date format: {e}. Use YYYY-MM-DD HH:MM"
                    ))
                },
            )?;

        let tz_info = self
            .timezone_manager
            .get_timezone_info(timezone)
            .ok_or_else(|| {
                TimezoneError::InvalidTimezone(format!("Timezone not found: {timezone}"))
            })?;

        let offset = FixedOffset::east_opt((tz_info.offset * 3600.0) as i32)
            .ok_or_else(|| TimezoneError::TimeConversionError("Invalid offset".to_string()))?;

        let local_datetime_with_offset = offset.from_local_datetime(&naive_datetime);

        match local_datetime_with_offset {
            LocalResult::Single(datetime) => Ok(datetime.with_timezone(&Utc)),
            LocalResult::Ambiguous(_, _) => Err(TimezoneError::TimeConversionError(
                "Ambiguous date/time (time change)".to_string(),
            )),
            LocalResult::None => Err(TimezoneError::TimeConversionError(
                "Invalid date/time for this timezone".to_string(),
            )),
        }
    }

    /// Convert UTC date to a specific timezone with proper date formatting
    pub fn format_from_utc_with_timezone(
        &self,
        utc_datetime: DateTime<Utc>,
        timezone: &str,
    ) -> Result<String> {
        let tz_info = self
            .timezone_manager
            .get_timezone_info(timezone)
            .ok_or_else(|| {
                TimezoneError::InvalidTimezone(format!("Timezone not found: {timezone}"))
            })?;

        let offset = FixedOffset::east_opt((tz_info.offset * 3600.0) as i32)
            .ok_or_else(|| TimezoneError::TimeConversionError("Invalid offset".to_string()))?;

        let local_datetime = utc_datetime.with_timezone(&offset);

        // Infer date format from timezone for proper display
        let date_format = self
            .geo_mapping_service
            .infer_date_format_from_timezone(timezone)
            .unwrap_or("YMD");

        let format_pattern = match date_format {
            "DMY" => "%d-%m-%Y %H:%M",
            "MDY" => "%m-%d-%Y %H:%M",
            "YMD" | _ => "%Y-%m-%d %H:%M",
        };

        Ok(local_datetime.format(format_pattern).to_string())
    }

    /// Format a UTC datetime using the user's preferred date format
    pub async fn format_from_utc_for_user(
        &self,
        utc_datetime: DateTime<Utc>,
        user_id: u64,
    ) -> Result<String> {
        let prefs = self
            .get_user_preferences(user_id)
            .await?
            .ok_or(TimezoneError::NotFound)?;

        let tz_info = self
            .timezone_manager
            .get_timezone_info(&prefs.timezone)
            .ok_or_else(|| {
                TimezoneError::InvalidTimezone(format!("Timezone not found: {}", prefs.timezone))
            })?;

        let offset = FixedOffset::east_opt((tz_info.offset * 3600.0) as i32)
            .ok_or_else(|| TimezoneError::TimeConversionError("Invalid offset".to_string()))?;

        let local_datetime = utc_datetime.with_timezone(&offset);

        // Format based on user's preferred date format
        let format_pattern = match prefs.date_format.as_deref() {
            Some("DMY") => "%d-%m-%Y %H:%M",
            Some("MDY") => "%m-%d-%Y %H:%M",
            Some("YMD") | None => "%Y-%m-%d %H:%M", // Default to YMD
            _ => "%Y-%m-%d %H:%M",                  // Fallback
        };

        Ok(local_datetime.format(format_pattern).to_string())
    }

    /// Gets the current time in the user's timezone with proper date formatting
    pub async fn get_current_time_for_user(&self, user_id: u64) -> Result<String> {
        let prefs = self
            .get_user_preferences(user_id)
            .await?
            .ok_or(TimezoneError::NotFound)?;

        let now_utc = Utc::now();
        self.format_from_utc_with_timezone_and_format(
            now_utc,
            &prefs.timezone,
            prefs.date_format.as_deref(),
        )
    }

    /// Gets the current time in a specific timezone with proper date formatting
    pub fn get_current_time_for_timezone(&self, timezone: &str) -> Result<String> {
        let now_utc = Utc::now();
        self.format_from_utc_with_timezone(now_utc, timezone)
    }

    /// Helper function to format UTC datetime with explicit date format
    fn format_from_utc_with_timezone_and_format(
        &self,
        utc_datetime: DateTime<Utc>,
        timezone: &str,
        date_format: Option<&str>,
    ) -> Result<String> {
        let tz_info = self
            .timezone_manager
            .get_timezone_info(timezone)
            .ok_or_else(|| {
                TimezoneError::InvalidTimezone(format!("Timezone not found: {timezone}"))
            })?;

        let offset = FixedOffset::east_opt((tz_info.offset * 3600.0) as i32)
            .ok_or_else(|| TimezoneError::TimeConversionError("Invalid offset".to_string()))?;

        let local_datetime = utc_datetime.with_timezone(&offset);

        // Use provided date format or infer from timezone
        let format_pattern = match date_format {
            Some("DMY") => "%d-%m-%Y %H:%M",
            Some("MDY") => "%m-%d-%Y %H:%M",
            Some("YMD") => "%Y-%m-%d %H:%M",
            None => {
                // Infer format from timezone if not provided
                let inferred_format = self
                    .geo_mapping_service
                    .infer_date_format_from_timezone(timezone)
                    .unwrap_or("YMD");
                match inferred_format {
                    "DMY" => "%d-%m-%Y %H:%M",
                    "MDY" => "%m-%d-%Y %H:%M",
                    "YMD" | _ => "%Y-%m-%d %H:%M",
                }
            }
            _ => "%Y-%m-%d %H:%M", // Fallback
        };

        Ok(local_datetime.format(format_pattern).to_string())
    }

    /// Search time zones by city, country, etc with geographic mapping
    pub fn search_timezones(&self, query: &str) -> Vec<&TimezoneInfo> {
        // First, try the timezone_manager search (handles special cases like US, Canada)
        let manager_results = self.timezone_manager.search_timezones(query);
        
        // If timezone_manager returns results, use them
        if !manager_results.is_empty() {
            return manager_results;
        }
        
        // Otherwise, try geo_mapping_service for direct country/city matches
        if let Some(timezone_id) = self.geo_mapping_service.search_geo_mapping(query) {
            if let Some(tz_info) = self.timezone_manager.get_timezone_info(timezone_id) {
                return vec![tz_info];
            }
        }
        
        // If nothing found, return empty
        vec![]
    }

    /// Get timezone information by exact name
    pub fn get_timezone_info(&self, timezone: &str) -> Option<&TimezoneInfo> {
        self.timezone_manager.get_timezone_info(timezone)
    }

    /// Parses and validates the entry of a task (single or weekly) based on the user's time zone
    pub async fn parse_task_input(
        &self,
        input_str: &str,
        task_type: &str,
        user_id: u64,
    ) -> std::result::Result<(Option<DateTime<Utc>>, Option<Recurrence>), String> {
        let user_timezone = self
            .get_user_timezone(user_id)
            .await
            .map_err(|e| format!("Error getting timezone: {e:?}"))?
            .ok_or("User has no timezone configured".to_string())?;

        // Get user's date format to parse the input correctly
        let user_date_format = self
            .get_user_date_format(user_id)
            .await
            .map_err(|e| format!("Error getting date format: {e:?}"))?
            .unwrap_or("YMD".to_string());

        match task_type {
            "single" => {
                // Determine the format pattern based on user's date format
                let format_pattern = match user_date_format.as_str() {
                    "DMY" => "%d-%m-%Y %H:%M",
                    "MDY" => "%m-%d-%Y %H:%M",
                    "YMD" | _ => "%Y-%m-%d %H:%M",
                };

                let naive_dt = chrono::NaiveDateTime::parse_from_str(input_str, format_pattern)
                    .map_err(|_| {
                        format!(
                            "Failed to parse date/time. Use format: {}",
                            match user_date_format.as_str() {
                                "DMY" => "DD-MM-YYYY HH:MM",
                                "MDY" => "MM-DD-YYYY HH:MM",
                                "YMD" | _ => "YYYY-MM-DD HH:MM",
                            }
                        )
                    })?;

                // Convert to the standard format for further processing
                let standard_format_str = naive_dt.format("%Y-%m-%d %H:%M").to_string();
                let utc_datetime = self
                    .parse_to_utc_with_timezone(&standard_format_str, &user_timezone)
                    .map_err(|e| format!("Error processing date/time: {e:?}"))?;

                let is_future = utc_datetime > Utc::now();

                if !is_future {
                    return Err("You cannot schedule a task in the past".into());
                }

                Ok((Some(utc_datetime), None))
            }
            "weekly" => {
                let (days, hour, minute) = Self::parse_weekly_input(input_str)?;

                let time_str = format!("{hour:02}:{minute:02}");
                let local_datetime_str = format!("1970-01-01 {time_str}");
                let utc_datetime = self
                    .parse_to_utc_with_timezone(&local_datetime_str, &user_timezone)
                    .map_err(|e| format!("Error processing time: {e:?}"))?;

                let recurrence = Recurrence::Weekly {
                    days,
                    hour: utc_datetime.time().hour() as u8,
                    minute: utc_datetime.time().minute() as u8,
                };

                Ok((None, Some(recurrence)))
            }
            _ => Err(format!("Unknown task type: {task_type}")),
        }
    }

    /// Parse weekly input string into weekdays, hour, and minute
    fn parse_weekly_input(input_str: &str) -> std::result::Result<(Vec<Weekday>, u8, u8), String> {
        let input = input_str.trim();
        let last_space = input.rfind(' ').ok_or("Invalid format. Use: days HH:MM")?;
        let days_str = &input[..last_space];
        let time_str = &input[last_space + 1..];

        let days = Self::parse_days(days_str)?;

        let (hour, minute) = Self::parse_time(time_str)?;

        Ok((days, hour, minute))
    }

    /// Parse days string into Weekday enums with intelligent parsing
    fn parse_days(days_str: &str) -> std::result::Result<Vec<Weekday>, String> {
        let mut days = Vec::new();

        for day_str in days_str.split(',') {
            let day_clean = day_str.trim();
            if let Some(weekday) = Weekday::from_str(day_clean) {
                days.push(weekday);
            } else {
                return Err(format!("Invalid day: {}", day_str));
            }
        }

        days.sort_by_key(|weekday| weekday.num_days_from_monday());
        Ok(days)
    }

    /// Parse time string into hour and minute
    fn parse_time(time_str: &str) -> std::result::Result<(u8, u8), String> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid time format. Use HH:MM".to_string());
        }

        let hour = parts[0]
            .parse::<u8>()
            .map_err(|_| "Invalid hour".to_string())?;
        let minute = parts[1]
            .parse::<u8>()
            .map_err(|_| "Invalid minute".to_string())?;

        if hour > 23 || minute > 59 {
            return Err("Invalid time values".to_string());
        }

        Ok((hour, minute))
    }

    /// Infer date format from a timezone string
    pub fn infer_date_format_from_timezone(&self, timezone: &str) -> Option<&'static str> {
        self.geo_mapping_service
            .infer_date_format_from_timezone(timezone)
    }
}

impl From<RepositoryError> for TimezoneError {
    fn from(error: RepositoryError) -> Self {
        TimezoneError::RepositoryError(error)
    }
}
