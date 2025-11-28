use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref COUNTRY_TO_TIMEZONE: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        // America Latina
        m.insert("argentina", "America/Argentina/Buenos_Aires");
        m.insert("brazil", "America/Sao_Paulo");
        m.insert("peru", "America/Lima");
        m.insert("mexico", "America/Mexico_City");
        m.insert("brasil", "America/Sao_Paulo");
        m.insert("chile", "America/Santiago");
        m.insert("colombia", "America/Bogota");
        m.insert("venezuela", "America/Caracas");
        m.insert("ecuador", "America/Guayaquil");
        m.insert("uruguay", "America/Montevideo");
        m.insert("paraguay", "America/Asuncion");
        m.insert("bolivia", "America/La_Paz");
        m.insert("costa rica", "America/Costa_Rica");
        m.insert("panama", "America/Panama");
        m.insert("republica dominicana", "America/Santo_Domingo");
        m.insert("dominican republic", "America/Santo_Domingo");
        m.insert("guatemala", "America/Guatemala");
        m.insert("honduras", "America/Tegucigalpa");
        m.insert("el salvador", "America/El_Salvador");
        m.insert("nicaragua", "America/Managua");
        m.insert("cuba", "America/Havana");
        m.insert("puerto rico", "America/Puerto_Rico");


        // Europa
        m.insert("spain", "Europe/Madrid");
        m.insert("espana", "Europe/Madrid");
        m.insert("france", "Europe/Paris");
        m.insert("germany", "Europe/Berlin");
        m.insert("italy", "Europe/Rome");
        m.insert("united kingdom", "Europe/London");
        m.insert("uk", "Europe/London");

        // Asia
        m.insert("japan", "Asia/Tokyo");
        m.insert("china", "Asia/Shanghai");
        m.insert("india", "Asia/Kolkata");

        m
    };

    static ref US_STATE_TO_TIMEZONE: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        m.insert("alabama", "America/New_York");
        m.insert("connecticut", "America/New_York");
        m.insert("delaware", "America/New_York");
        m.insert("florida", "America/New_York");
        m.insert("georgia", "America/New_York");
        m.insert("indiana", "America/New_York");
        m.insert("kentucky", "America/New_York");
        m.insert("maine", "America/New_York");
        m.insert("maryland", "America/New_York");
        m.insert("massachusetts", "America/New_York");
        m.insert("michigan", "America/New_York");
        m.insert("new hampshire", "America/New_York");
        m.insert("new jersey", "America/New_York");
        m.insert("new york", "America/New_York");
        m.insert("north carolina", "America/New_York");
        m.insert("ohio", "America/New_York");
        m.insert("pennsylvania", "America/New_York");
        m.insert("rhode island", "America/New_York");
        m.insert("south carolina", "America/New_York");
        m.insert("tennessee", "America/New_York");
        m.insert("vermont", "America/New_York");
        m.insert("virginia", "America/New_York");
        m.insert("west virginia", "America/New_York");
        m.insert("district of columbia", "America/New_York");
        m.insert("washington dc", "America/New_York");
        m.insert("arkansas", "America/Chicago");
        m.insert("illinois", "America/Chicago");
        m.insert("iowa", "America/Chicago");
        m.insert("kansas", "America/Chicago");
        m.insert("louisiana", "America/Chicago");
        m.insert("minnesota", "America/Chicago");
        m.insert("mississippi", "America/Chicago");
        m.insert("missouri", "America/Chicago");
        m.insert("nebraska", "America/Chicago");
        m.insert("north dakota", "America/Chicago");
        m.insert("oklahoma", "America/Chicago");
        m.insert("south dakota", "America/Chicago");
        m.insert("texas", "America/Chicago");
        m.insert("wisconsin", "America/Chicago");
        m.insert("arizona", "America/Phoenix");
        m.insert("colorado", "America/Denver");
        m.insert("idaho", "America/Denver");
        m.insert("montana", "America/Denver");
        m.insert("new mexico", "America/Denver");
        m.insert("utah", "America/Denver");
        m.insert("wyoming", "America/Denver");
        m.insert("california", "America/Los_Angeles");
        m.insert("nevada", "America/Los_Angeles");
        m.insert("oregon", "America/Los_Angeles");
        m.insert("washington", "America/Los_Angeles");
        m.insert("alaska", "America/Anchorage");
        m.insert("hawaii", "Pacific/Honolulu");

        m
    };

    static ref CANADA_PROVINCE_TO_TIMEZONE: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        m.insert("british columbia", "America/Vancouver");
        m.insert("columbia britanica", "America/Vancouver");
        m.insert("alberta", "America/Edmonton");
        m.insert("northwest territories", "America/Yellowknife");
        m.insert("saskatchewan", "America/Regina");
        m.insert("manitoba", "America/Winnipeg");
        m.insert("ontario", "America/Toronto");
        m.insert("quebec", "America/Toronto");
        m.insert("new brunswick", "America/Halifax");
        m.insert("nova scotia", "America/Halifax");
        m.insert("prince edward island", "America/Halifax");
        m.insert("newfoundland", "America/St_Johns");
        m.insert("labrador", "America/St_Johns");
        m.insert("newfoundland and labrador", "America/St_Johns");
        m.insert("nunavut", "America/Iqaluit");
        m.insert("yukon", "America/Whitehorse");

        m
    };

    static ref COUNTRY_TO_DATE_FORMAT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        // DMY format (Day-Month-Year) - Europe, Latin America
        m.insert("argentina", "DMY");
        m.insert("brazil", "DMY");
        m.insert("brasil", "DMY");
        m.insert("spain", "DMY");
        m.insert("espana", "DMY");
        m.insert("france", "DMY");
        m.insert("germany", "DMY");
        m.insert("italy", "DMY");
        m.insert("united kingdom", "DMY");
        m.insert("uk", "DMY");
        m.insert("peru", "DMY");
        m.insert("mexico", "DMY");
        m.insert("chile", "DMY");
        m.insert("colombia", "DMY");
        m.insert("venezuela", "DMY");
        m.insert("ecuador", "DMY");
        m.insert("uruguay", "DMY");
        m.insert("paraguay", "DMY");
        m.insert("bolivia", "DMY");
        m.insert("costa rica", "DMY");
        m.insert("panama", "DMY");
        m.insert("republica dominicana", "DMY");
        m.insert("dominican republic", "DMY");
        m.insert("guatemala", "DMY");
        m.insert("honduras", "DMY");
        m.insert("el salvador", "DMY");
        m.insert("nicaragua", "DMY");
        m.insert("cuba", "DMY");
        m.insert("puerto rico", "DMY");

        // MDY format (Month-Day-Year) - United States, Canada
        m.insert("canada", "MDY");
        m.insert("canadá", "MDY");
        m.insert("united states", "MDY");
        m.insert("usa", "MDY");
        m.insert("us", "MDY");

        // YMD format (Year-Month-Day) - Asia, International standards
        m.insert("japan", "YMD");
        m.insert("china", "YMD");
        m.insert("india", "YMD");
        m.insert("korea", "YMD");
        m.insert("south korea", "YMD");
        m.insert("north korea", "YMD");
        m.insert("taiwan", "YMD");
        m.insert("hong kong", "YMD");
        m.insert("singapore", "YMD");

        m
    };

    static ref US_STATE_TO_DATE_FORMAT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        // All US states use MDY format
        m.insert("alabama", "MDY");
        m.insert("connecticut", "MDY");
        m.insert("delaware", "MDY");
        m.insert("florida", "MDY");
        m.insert("georgia", "MDY");
        m.insert("indiana", "MDY");
        m.insert("kentucky", "MDY");
        m.insert("maine", "MDY");
        m.insert("maryland", "MDY");
        m.insert("massachusetts", "MDY");
        m.insert("michigan", "MDY");
        m.insert("new hampshire", "MDY");
        m.insert("new jersey", "MDY");
        m.insert("new york", "MDY");
        m.insert("north carolina", "MDY");
        m.insert("ohio", "MDY");
        m.insert("pennsylvania", "MDY");
        m.insert("rhode island", "MDY");
        m.insert("south carolina", "MDY");
        m.insert("tennessee", "MDY");
        m.insert("vermont", "MDY");
        m.insert("virginia", "MDY");
        m.insert("west virginia", "MDY");
        m.insert("district of columbia", "MDY");
        m.insert("washington dc", "MDY");
        m.insert("arkansas", "MDY");
        m.insert("illinois", "MDY");
        m.insert("iowa", "MDY");
        m.insert("kansas", "MDY");
        m.insert("louisiana", "MDY");
        m.insert("minnesota", "MDY");
        m.insert("mississippi", "MDY");
        m.insert("missouri", "MDY");
        m.insert("nebraska", "MDY");
        m.insert("north dakota", "MDY");
        m.insert("oklahoma", "MDY");
        m.insert("south dakota", "MDY");
        m.insert("texas", "MDY");
        m.insert("wisconsin", "MDY");
        m.insert("arizona", "MDY");
        m.insert("colorado", "MDY");
        m.insert("idaho", "MDY");
        m.insert("montana", "MDY");
        m.insert("new mexico", "MDY");
        m.insert("utah", "MDY");
        m.insert("wyoming", "MDY");
        m.insert("california", "MDY");
        m.insert("nevada", "MDY");
        m.insert("oregon", "MDY");
        m.insert("washington", "MDY");
        m.insert("alaska", "MDY");
        m.insert("hawaii", "MDY");

        m
    };

    static ref CANADA_PROVINCE_TO_DATE_FORMAT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        // All Canadian provinces use MDY format
        m.insert("british columbia", "MDY");
        m.insert("columbia britanica", "MDY");
        m.insert("alberta", "MDY");
        m.insert("northwest territories", "MDY");
        m.insert("saskatchewan", "MDY");
        m.insert("manitoba", "MDY");
        m.insert("ontario", "MDY");
        m.insert("quebec", "MDY");
        m.insert("new brunswick", "MDY");
        m.insert("nova scotia", "MDY");
        m.insert("prince edward island", "MDY");
        m.insert("newfoundland", "MDY");
        m.insert("labrador", "MDY");
        m.insert("newfoundland and labrador", "MDY");
        m.insert("nunavut", "MDY");
        m.insert("yukon", "MDY");

        m
    };
}

pub struct GeoMappingService;

impl GeoMappingService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_timezone_for_country(&self, country: &str) -> Option<&'static str> {
        COUNTRY_TO_TIMEZONE
            .get(&country.to_lowercase().as_str())
            .copied()
    }

    pub fn get_timezone_for_state(&self, state: &str) -> Option<&'static str> {
        US_STATE_TO_TIMEZONE
            .get(&state.to_lowercase().as_str())
            .copied()
    }

    pub fn get_timezone_for_canada_province(&self, province: &str) -> Option<&'static str> {
        CANADA_PROVINCE_TO_TIMEZONE
            .get(&province.to_lowercase().as_str())
            .copied()
    }

    pub fn search_geo_mapping(&self, query: &str) -> Option<&'static str> {
        let query_lower = query.to_lowercase();

        // search first in countries
        if let Some(tz) = self.get_timezone_for_country(&query_lower) {
            return Some(tz);
        }

        // then search in US states
        if let Some(tz) = self.get_timezone_for_state(&query_lower) {
            return Some(tz);
        }

        // then search in Canada provinces
        if let Some(tz) = self.get_timezone_for_canada_province(&query_lower) {
            return Some(tz);
        }

        None
    }

    /// Infer date format from timezone string
    pub fn infer_date_format_from_timezone(&self, timezone: &str) -> Option<&'static str> {
        let parts: Vec<&str> = timezone.split('/').collect();
        if parts.len() >= 2 {
            let region = parts[0].to_lowercase();
            let location = parts[1].to_lowercase().replace('_', " ");

            // Check specific country mappings by country name in the timezone
            if location.contains("argentina") {
                return Some("DMY");
            } else if location.contains("brazil") || location.contains("brasil") {
                return Some("DMY");
            } else if location.contains("mexico") {
                return Some("DMY");
            } else if location.contains("chile") {
                return Some("DMY");
            } else if location.contains("colombia") {
                return Some("DMY");
            } else if location.contains("peru") {
                return Some("DMY");
            } else if location.contains("venezuela") {
                return Some("DMY");
            } else if location.contains("ecuador") {
                return Some("DMY");
            } else if location.contains("uruguay") {
                return Some("DMY");
            } else if location.contains("paraguay") {
                return Some("DMY");
            } else if location.contains("bolivia") {
                return Some("DMY");
            }

            // Check region-based mappings
            if region == "europe" {
                return Some("DMY");
            } else if region == "asia" {
                return Some("YMD");
            } else if region == "america" {
                // For US/Canada, use MDY - check by city names
                if location.contains("new york")
                    || location.contains("los angeles")
                    || location.contains("chicago")
                    || location.contains("denver")
                    || location.contains("toronto")
                    || location.contains("vancouver")
                    || location.contains("winnipeg")
                    || location.contains("halifax")
                {
                    return Some("MDY");
                }
                // For Latin America cities (fallback), use DMY
                else if location.contains("buenos aires")
                    || location.contains("sao paulo")
                    || location.contains("lima")
                    || location.contains("bogota")
                    || location.contains("santiago")
                    || location.contains("caracas")
                    || location.contains("quito")
                    || location.contains("montevideo")
                    || location.contains("asuncion")
                    || location.contains("la paz")
                {
                    return Some("DMY");
                }
            }
        }

        // Default to YMD (international standard)
        Some("YMD")
    }
}
