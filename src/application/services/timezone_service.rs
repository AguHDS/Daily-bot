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

    /// Set the time zone for a user
    pub async fn set_user_timezone(&self, user_id: u64, timezone_str: &str) -> Result<()> {
        if !self.timezone_manager.validate_timezone(timezone_str) {
            return Err(TimezoneError::InvalidTimezone(format!(
                "Invalid timezone: {timezone_str}"
            )));
        }

        let preferences = match self.user_prefs_repo.get(user_id).await {
            Ok(Some(mut prefs)) => {
                prefs.update_timezone(timezone_str.to_string());
                prefs
            }
            Ok(None) => UserPreferences::new(user_id, timezone_str.to_string()),
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

    /// Convert a local date/time to UTC using the user's timezone
    pub async fn parse_to_utc(&self, local_datetime: &str, user_id: u64) -> Result<DateTime<Utc>> {
        let timezone_str = self
            .get_user_timezone(user_id)
            .await?
            .ok_or(TimezoneError::NotFound)?;

        self.parse_to_utc_with_timezone(local_datetime, &timezone_str)
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

    /// Convert UTC date to a specific timezone
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

        Ok(local_datetime.format("%Y-%m-%d %H:%M").to_string())
    }

    /// Gets the current time in the user's timezone
    pub async fn get_current_time_for_user(&self, user_id: u64) -> Result<String> {
        let timezone_str = self
            .get_user_timezone(user_id)
            .await?
            .ok_or(TimezoneError::NotFound)?;

        self.get_current_time_for_timezone(&timezone_str)
    }

    /// Gets the current time in a specific timezone
    pub fn get_current_time_for_timezone(&self, timezone: &str) -> Result<String> {
        let now_utc = Utc::now();
        self.format_from_utc_with_timezone(now_utc, timezone)
    }

    /// Search time zones by city, country, etc with geographic mapping
    pub fn search_timezones(&self, query: &str) -> Vec<&TimezoneInfo> {
        if let Some(timezone_id) = self.geo_mapping_service.search_geo_mapping(query) {
            if let Some(tz_info) = self.timezone_manager.get_timezone_info(timezone_id) {
                return vec![tz_info];
            }
        }
        self.timezone_manager.search_timezones(query)
    }

    /// Validates if a local date/time is not in the past
    pub async fn is_future_datetime(&self, local_datetime: &str, user_id: u64) -> Result<bool> {
        let utc_datetime = self.parse_to_utc(local_datetime, user_id).await?;
        let now_utc = Utc::now();
        Ok(utc_datetime > now_utc)
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

        match task_type {
            "single" => {
                let naive_dt = chrono::NaiveDateTime::parse_from_str(input_str, "%Y-%m-%d %H:%M")
                    .map_err(|_| {
                    "Failed to parse date/time. Use format: YYYY-MM-DD HH:MM".to_string()
                })?;

                let local_datetime_str = naive_dt.format("%Y-%m-%d %H:%M").to_string();
                let utc_datetime = self
                    .parse_to_utc_with_timezone(&local_datetime_str, &user_timezone)
                    .map_err(|e| format!("Error processing date/time: {e:?}"))?;

                let is_future = self
                    .is_future_datetime(&local_datetime_str, user_id)
                    .await
                    .map_err(|e| format!("Error validating date: {e:?}"))?;

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
}

impl From<RepositoryError> for TimezoneError {
    fn from(error: RepositoryError) -> Self {
        TimezoneError::RepositoryError(error)
    }
}
