use chrono::{DateTime, FixedOffset, LocalResult, TimeZone, Utc};
use std::sync::Arc;

use super::geo_mapping_service::GeoMappingService;
use crate::domain::entities::user_preferences::UserPreferences;
use crate::domain::repositories::user_preferences_repository::{
    RepositoryError, UserPreferencesRepository,
};
use crate::infrastructure::timezone::timezone_manager::{TimezoneInfo, TimezoneManager};

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
        // validate that the timezone exists
        if !self.timezone_manager.validate_timezone(timezone_str) {
            return Err(TimezoneError::InvalidTimezone(format!(
                "Invalid timezone: {}",
                timezone_str
            )));
        }

        // create or update user preferences
        let preferences = match self.user_prefs_repo.get(user_id).await {
            Ok(Some(mut prefs)) => {
                prefs.update_timezone(timezone_str.to_string());
                prefs
            }
            Ok(None) => UserPreferences::new(user_id, timezone_str.to_string()),
            Err(e) => return Err(TimezoneError::RepositoryError(e)),
        };

        // Guardar las preferencias
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

    /// Convert a local date/time to UTC using a specific timezone
    pub fn parse_to_utc_with_timezone(
        &self,
        local_datetime: &str,
        timezone: &str,
    ) -> Result<DateTime<Utc>> {
        // parse local date/time (format: "YYYY-MM-DD HH:MM")
        let naive_datetime =
            chrono::NaiveDateTime::parse_from_str(local_datetime, "%Y-%m-%d %H:%M").map_err(
                |e| {
                    TimezoneError::TimeConversionError(format!(
                        "Invalid date format: {}. Use YYYY-MM-DD HH:MM",
                        e
                    ))
                },
            )?;

        // get timezone information
        let tz_info = self
            .timezone_manager
            .get_timezone_info(timezone)
            .ok_or_else(|| {
                TimezoneError::InvalidTimezone(format!("Timezone not found: {}", timezone))
            })?;

        // create datetime with offset
        let offset = FixedOffset::east_opt((tz_info.offset * 3600.0) as i32)
            .ok_or_else(|| TimezoneError::TimeConversionError("Invalid offset".to_string()))?;

        let local_datetime_with_offset = offset.from_local_datetime(&naive_datetime);

        match local_datetime_with_offset {
            LocalResult::Single(datetime) => {
                // convert to UTC
                Ok(datetime.with_timezone(&Utc))
            }
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
                TimezoneError::InvalidTimezone(format!("Timezone not found: {}", timezone))
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
        // 1. Primero buscar en el mapeo geográfico (países/estados)
        if let Some(timezone_id) = self.geo_mapping_service.search_geo_mapping(query) {
            if let Some(tz_info) = self.timezone_manager.get_timezone_info(timezone_id) {
                return vec![tz_info]; // Devolver resultado directo del mapeo
            }
        }

        // if there is no geographic mapping, search normally
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
}

impl From<RepositoryError> for TimezoneError {
    fn from(error: RepositoryError) -> Self {
        TimezoneError::RepositoryError(error)
    }
}
