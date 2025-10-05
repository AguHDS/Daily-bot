use serenity::all::{CommandDataOption, CommandDataOptionValue};

/// Extract a string option from a slash command by index
pub fn get_string_option(options: &Vec<CommandDataOption>, index: usize) -> Option<String> {
    options.get(index).and_then(|opt| {
        if let CommandDataOptionValue::String(s) = &opt.value {
            Some(s.clone())
        } else {
            None
        }
    })
}
