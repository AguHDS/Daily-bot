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
        let data_path = "./data/timezones.json";
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

    /// Search timezones by city, country, using fuzzy matching with prioritization
    pub fn search_timezones(&self, query: &str) -> Vec<&TimezoneInfo> {
        let query_lower = query.to_lowercase().trim().to_string();
        let mut results = Vec::new();

        // SPECIAL CASE: US timezones
        if query_lower == "america"
            || query_lower == "usa"
            || query_lower == "us"
            || query_lower == "united states"
        {
            return self.search_us_timezones();
        }

        // SPECIAL CASE: Canada timezones
        if query_lower == "canada" || query_lower == "canadá" {
            return self.search_canada_timezones();
        }

        // SPECIAL CASE: North America - both US and Canada
        if query_lower == "north america" {
            return self.search_north_america_timezones();
        }

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

        // fuzzy search with prioritization
        let mut fuzzy_results: Vec<(i64, &TimezoneInfo, u8)> = Vec::new(); // (score, tz_info, priority)

        for tz_info in self.timezones.values() {
            let mut priority = 0;

            // Priority 1: Exact match in value (timezone name)
            let value_score = self
                .fuzzy_matcher
                .fuzzy_match(&tz_info.value.to_lowercase(), &query_lower);
            if let Some(score) = value_score {
                if score > 50 {
                    // Good match threshold
                    priority = 3;
                }
                fuzzy_results.push((score, tz_info, priority));
            }

            // Priority 2: Search in UTC cities (higher priority for direct matches)
            for utc_tz in &tz_info.utc {
                if let Some(city_name) = Self::extract_city_name(utc_tz) {
                    let city_score = self
                        .fuzzy_matcher
                        .fuzzy_match(&city_name.to_lowercase(), &query_lower);
                    if let Some(score) = city_score {
                        let city_priority = if score > 70 { 4 } else { 2 };
                        fuzzy_results.push((score, tz_info, city_priority));
                    }
                }
            }

            // Priority 3: Search in descriptive text (lower priority)
            let text_score = self
                .fuzzy_matcher
                .fuzzy_match(&tz_info.text.to_lowercase(), &query_lower);
            if let Some(score) = text_score {
                // Lower priority for text matches to avoid generic results like "Central America"
                let text_priority = if score > 60 && !self.is_generic_timezone(tz_info) {
                    1
                } else {
                    0
                };
                fuzzy_results.push((score, tz_info, text_priority));
            }
        }

        // Sort by priority first, then by score
        fuzzy_results.sort_by(|a, b| {
            b.2.cmp(&a.2) // Higher priority first
                .then(b.0.cmp(&a.0)) // Then higher score
        });

        for (_, tz_info, _) in fuzzy_results {
            if !results.contains(&tz_info) {
                results.push(tz_info);
                // Limit to 8 results for better UX
                if results.len() >= 8 {
                    break;
                }
            }
        }

        results
    }

    /// Special search for US timezones only
    fn search_us_timezones(&self) -> Vec<&TimezoneInfo> {
        let mut results = Vec::new();
        let mut seen_values = std::collections::HashSet::new();

        let us_utc_timezones = [
            // Eastern Time Zone
            "America/New_York",
            "America/Detroit",
            "America/Indiana/Indianapolis",
            "America/Indiana/Marengo",
            "America/Indiana/Vevay",
            "America/Louisville",
            
            // Central Time Zone
            "America/Chicago",
            "America/Menominee",
            "America/Indiana/Knox",
            
            // Mountain Time Zone
            "America/Denver",
            "America/Boise",
            
            // Mountain Standard (Arizona - no DST)
            "America/Phoenix",
            
            // Pacific Time Zone
            "America/Los_Angeles",
            
            // Alaska Time Zone
            "America/Anchorage",
            "America/Juneau",
            
            // Hawaii
            "Pacific/Honolulu",
        ];

        for utc_tz in &us_utc_timezones {
            if let Some(tz_info) = self.timezones.get(*utc_tz) {
                if seen_values.insert(&tz_info.value) {
                    results.push(tz_info);
                }
            }
        }

        results.sort_by(|a, b| {
            b.offset
                .partial_cmp(&a.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Special search for Canada timezones only
    fn search_canada_timezones(&self) -> Vec<&TimezoneInfo> {
        let mut results = Vec::new();
        let mut seen_values = std::collections::HashSet::new();

        let canada_utc_timezones = [
            // Newfoundland Time (UTC-3:30) - Unique Canadian timezone
            "America/St_Johns",
            
            // Atlantic Time Zone (Canada)
            "America/Halifax",
            "America/Moncton",
            "America/Glace_Bay",
            
            // Eastern Time Zone (shared with US)
            "America/Toronto",
            "America/Montreal",
            "America/Iqaluit",
            
            // Central Time Zone (shared with US)
            "America/Winnipeg",
            
            // Saskatchewan (Central Standard - no DST)
            "America/Regina",
            
            // Mountain Time Zone (shared with US)
            "America/Edmonton",
            
            // Pacific Time Zone (shared with US)
            "America/Vancouver",
        ];

        for utc_tz in &canada_utc_timezones {
            if let Some(tz_info) = self.timezones.get(*utc_tz) {
                if seen_values.insert(&tz_info.value) {
                    results.push(tz_info);
                }
            }
        }

        results.sort_by(|a, b| {
            b.offset
                .partial_cmp(&a.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Special search for all North American timezones (US + Canada)
    fn search_north_america_timezones(&self) -> Vec<&TimezoneInfo> {
        let mut results = Vec::new();
        let mut seen_values = std::collections::HashSet::new();

        // Combine both US and Canada timezones
        let north_america_utc_timezones = [
            // Canada - Newfoundland
            "America/St_Johns",
            // Canada - Atlantic
            "America/Halifax",
            "America/Moncton",
            // US/Canada - Eastern
            "America/New_York",
            "America/Toronto",
            "America/Detroit",
            "America/Indiana/Indianapolis",
            // US/Canada - Central
            "America/Chicago",
            "America/Winnipeg",
            // Canada - Saskatchewan
            "America/Regina",
            // US/Canada - Mountain
            "America/Denver",
            "America/Edmonton",
            "America/Boise",
            // US - Arizona
            "America/Phoenix",
            // US/Canada - Pacific
            "America/Los_Angeles",
            "America/Vancouver",
            // US - Alaska
            "America/Anchorage",
            "America/Juneau",
            // US - Hawaii
            "Pacific/Honolulu",
        ];

        for utc_tz in &north_america_utc_timezones {
            if let Some(tz_info) = self.timezones.get(*utc_tz) {
                if seen_values.insert(&tz_info.value) {
                    results.push(tz_info);
                }
            }
        }

        results.sort_by(|a, b| {
            b.offset
                .partial_cmp(&a.offset)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Check if a timezone is too generic (like "Central America" when searching for US)
    fn is_generic_timezone(&self, tz_info: &TimezoneInfo) -> bool {
        let generic_terms = [
            "central america",
            "south america",
            "latin america",
            "caribbean",
            "generic",
            "etc/gmt",
        ];

        let text_lower = tz_info.text.to_lowercase();
        generic_terms.iter().any(|term| text_lower.contains(term))
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
