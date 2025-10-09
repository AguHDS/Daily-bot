use fuzzy_matcher::FuzzyMatcher;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TimezoneData {
    pub timezones: Vec<TimezoneInfo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TimezoneInfo {
    pub value: String,
    pub abbr: String,
    pub offset: f32, // hours from UTC
    pub isdst: bool,
    pub text: String,
    pub utc: Vec<String>,
}

pub struct TimezoneManager {
    timezones: HashMap<String, TimezoneInfo>,
    city_to_timezone: HashMap<String, Vec<String>>, // city ​​-> timezone list
    fuzzy_matcher: fuzzy_matcher::skim::SkimMatcherV2,
}

impl TimezoneManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_path = "src/infrastructure/data/timezones.json";
        let file_content = fs::read_to_string(data_path)?;

        let timezones_list: Vec<TimezoneInfo> = serde_json::from_str(&file_content)?;

        let mut timezones = HashMap::new();
        let mut city_to_timezone = HashMap::new();

        // index timezones and cities
        for tz_info in timezones_list {
            // index by each UTC timezone
            for utc_tz in &tz_info.utc {
                timezones.insert(utc_tz.clone(), tz_info.clone());
            }

            // index cities from UTC names
            for utc_tz in &tz_info.utc {
                if let Some(city_name) = Self::extract_city_name(utc_tz) {
                    city_to_timezone
                        .entry(city_name.to_lowercase())
                        .or_insert_with(Vec::new)
                        .push(utc_tz.clone());
                }
            }

            timezones.insert(tz_info.value.clone(), tz_info.clone());
        }

        Ok(Self {
            timezones,
            city_to_timezone,
            fuzzy_matcher: fuzzy_matcher::skim::SkimMatcherV2::default(),
        })
    }

    /// Extracts the city name from a timezone string (ex: "America/New_York" => "New York")
    fn extract_city_name(utc_tz: &str) -> Option<String> {
        let parts: Vec<&str> = utc_tz.split('/').collect();
        if parts.len() >= 2 {
            let city_part = parts.last().unwrap();
            Some(city_part.replace('_', " "))
        } else {
            None
        }
    }

    /// Search timezones by city, country, using fuzzy matching
    pub fn search_timezones(&self, query: &str) -> Vec<&TimezoneInfo> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // exact search in cities first
        if let Some(timezone_names) = self.city_to_timezone.get(&query_lower) {
            for tz_name in timezone_names {
                if let Some(tz_info) = self.timezones.get(tz_name) {
                    if !results.contains(&tz_info) {
                        results.push(tz_info);
                    }
                }
            }
        }

        // if we already have exact results, return them
        if !results.is_empty() {
            return results;
        }

        // fuzzy search in all timezones and cities
        let mut fuzzy_results: Vec<(i64, &TimezoneInfo)> = Vec::new();

        for tz_info in self.timezones.values() {
            // search in the descriptive text
            let text_score = self
                .fuzzy_matcher
                .fuzzy_match(&tz_info.text.to_lowercase(), &query_lower);
            if let Some(score) = text_score {
                fuzzy_results.push((score, tz_info));
            }

            // search in UTC cities
            for utc_tz in &tz_info.utc {
                if let Some(city_name) = Self::extract_city_name(utc_tz) {
                    let city_score = self
                        .fuzzy_matcher
                        .fuzzy_match(&city_name.to_lowercase(), &query_lower);
                    if let Some(score) = city_score {
                        fuzzy_results.push((score, tz_info));
                    }
                }
            }

            // search in the value (principal name)
            let value_score = self
                .fuzzy_matcher
                .fuzzy_match(&tz_info.value.to_lowercase(), &query_lower);
            if let Some(score) = value_score {
                fuzzy_results.push((score, tz_info));
            }
        }

        // sort by score (highest first) and remove duplicates
        fuzzy_results.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, tz_info) in fuzzy_results {
            if !results.contains(&tz_info) {
                results.push(tz_info);
                // Limitar a 10 resultados
                if results.len() >= 10 {
                    break;
                }
            }
        }

        results
    }

    /// Get time zone information by exact name
    pub fn get_timezone_info(&self, timezone: &str) -> Option<&TimezoneInfo> {
        self.timezones.get(timezone)
    }

    /// Check if a timezone exists
    pub fn validate_timezone(&self, timezone: &str) -> bool {
        self.timezones.contains_key(timezone)
    }
}

impl Default for TimezoneManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize TimezoneManager")
    }
}
