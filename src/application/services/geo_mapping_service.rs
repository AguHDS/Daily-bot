use async_trait::async_trait;
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

        // Canada
        m.insert("canada", "America/Toronto");
        m.insert("canadá", "America/Toronto");

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
}

// For future migration to MySQL
#[allow(dead_code)]
#[async_trait]
pub trait GeoMappingRepository: Send + Sync {
    async fn get_timezone_for_country(&self, country: &str) -> Option<String>;
    async fn get_timezone_for_state(&self, state: &str) -> Option<String>;
    async fn get_timezone_for_canada_province(&self, province: &str) -> Option<String>;
}
