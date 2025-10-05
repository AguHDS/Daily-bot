use chrono::Weekday;

/// Parse a weekly input string into weekdays, hour, minute, and formatted string
pub fn parse_weekly_input(input: &str) -> Result<(Vec<Weekday>, u8, u8, String), Box<dyn std::error::Error>> {
    // separate days and time
    let input = input.trim();
    let last_space = input.rfind(' ').ok_or("Invalid format. Use: days HH:MM")?;
    let days_str = &input[..last_space];
    let time_str = &input[last_space + 1..];

    // mapping of day names to chrono::Weekday
    let day_map = vec![
        ("monday", ("Mon", Weekday::Mon)),
        ("tuesday", ("Tue", Weekday::Tue)),
        ("wednesday", ("Wed", Weekday::Wed)),
        ("thursday", ("Thu", Weekday::Thu)),
        ("friday", ("Fri", Weekday::Fri)),
        ("saturday", ("Sat", Weekday::Sat)),
        ("sunday", ("Sun", Weekday::Sun)),
        ("mon", ("Mon", Weekday::Mon)),
        ("tue", ("Tue", Weekday::Tue)),
        ("wed", ("Wed", Weekday::Wed)),
        ("thu", ("Thu", Weekday::Thu)),
        ("fri", ("Fri", Weekday::Fri)),
        ("sat", ("Sat", Weekday::Sat)),
        ("sun", ("Sun", Weekday::Sun)),
    ];

    let mut weekdays: Vec<Weekday> = Vec::new();
    let mut day_abbrevs: Vec<String> = Vec::new();

    // parse days
    for day in days_str.split(',') {
        let day_clean = day.trim().to_lowercase();
        let mut found = false;
        for (name, (abbr, weekday)) in &day_map {
            if day_clean == *name {
                weekdays.push(*weekday);
                day_abbrevs.push(abbr.to_string());
                found = true;
                break;
            }
        }
        if !found {
            return Err(format!("Invalid weekday: {}", day).into());
        }
    }

    // order days from Monday to Sunday
    let mut combined: Vec<(&Weekday, &String)> = weekdays.iter().zip(day_abbrevs.iter()).collect();
    combined.sort_by_key(|(weekday, _)| weekday.num_days_from_monday());
    let (sorted_weekdays, sorted_abbrevs): (Vec<Weekday>, Vec<&String>) =
        combined.into_iter().map(|(w, a)| (*w, a)).unzip();

    let weekdays = sorted_weekdays;
    let day_abbrevs: Vec<String> = sorted_abbrevs.into_iter().map(|s| s.to_string()).collect();

    // parse hour
    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 2 {
        return Err("Invalid time format. Use HH:MM".into());
    }
    let hour: u8 = time_parts[0].parse()?;
    let minute: u8 = time_parts[1].parse()?;

    let formatted = format!("{} {}", day_abbrevs.join(","), time_str);

    Ok((weekdays, hour, minute, formatted))
}
