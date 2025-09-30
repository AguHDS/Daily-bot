use crate::application::domain::task::Task;
use serde_json;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

// Save to JSON
pub fn save_tasks(tasks: &Vec<Task>, file_path: &str) -> io::Result<()> {
    let json = serde_json::to_string_pretty(tasks)?;
    let mut file = fs::File::create(file_path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

// Load to JSON
pub fn load_tasks(file_path: &str) -> io::Result<Vec<Task>> {
    if !Path::new(file_path).exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(file_path)?;
    let tasks: Vec<Task> = serde_json::from_str(&data)?;
    Ok(tasks)
}
