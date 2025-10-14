use chrono::Weekday;

pub trait WeekdayFormat {
    fn to_short_en(&self) -> &'static str;
    fn from_str(s: &str) -> Option<Weekday>;
}

impl WeekdayFormat for Weekday {
    /// Converts weekday to short english abbreviation
    fn to_short_en(&self) -> &'static str {
        match self {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }

    /// Parses string representation into Weekday enum
    fn from_str(s: &str) -> Option<Weekday> {
        match s.to_lowercase().as_str() {
            "monday" | "mon" => Some(Weekday::Mon),
            "tuesday" | "tue" => Some(Weekday::Tue),
            "wednesday" | "wed" => Some(Weekday::Wed),
            "thursday" | "thu" => Some(Weekday::Thu),
            "friday" | "fri" => Some(Weekday::Fri),
            "saturday" | "sat" => Some(Weekday::Sat),
            "sunday" | "sun" => Some(Weekday::Sun),
            _ => None,
        }
    }
}
