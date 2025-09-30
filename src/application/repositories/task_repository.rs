use crate::application::domain::Task;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory repository for tasks
#[derive(Clone, Default)]
pub struct TaskRepository {
    tasks: Arc<Mutex<HashMap<u64, Task>>>, // id -> Task
    next_id: Arc<Mutex<u64>>,              // for auto-incrementing IDs
}

impl TaskRepository {
    const FILE_PATH: &'static str = "tasks.json";

    // Save all tasks to JSON file
    pub fn save_all(&self) -> std::io::Result<()> {
        let tasks = self.tasks.lock().unwrap();
        let all_tasks: Vec<Task> = tasks.values().cloned().collect();

        crate::application::repositories::json_storage::save_tasks(&all_tasks, Self::FILE_PATH)
    }

    // Load all tasks from JSON file
    pub fn load_all(&self) -> std::io::Result<()> {
        let loaded_tasks =
            crate::application::repositories::json_storage::load_tasks(Self::FILE_PATH)?;

        let mut tasks = self.tasks.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        for task in loaded_tasks {
            if task.id >= *next_id {
                *next_id = task.id + 1;
            }
            tasks.insert(task.id, task);
        }

        Ok(())
    }

    // Create new empty repository
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn add_task(
        &self,
        user_id: u64,
        message: String,
        scheduled_time: DateTime<Utc>,
        repeat_daily: bool,
    ) -> u64 {
        let mut id_lock = self.next_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;

        let task = Task::new(id, user_id, message, scheduled_time, repeat_daily);
        self.tasks.lock().unwrap().insert(id, task);

        id
    }

    pub fn list_tasks(&self) -> Vec<Task> {
        self.tasks.lock().unwrap().values().cloned().collect()
    }

    pub fn get_task(&self, id: u64) -> Option<Task> {
        self.tasks.lock().unwrap().get(&id).cloned()
    }

    pub fn complete_task(&self, id: u64) -> bool {
        if let Some(task) = self.tasks.lock().unwrap().get_mut(&id) {
            task.completed = true;
            true
        } else {
            false
        }
    }

    pub fn remove_task(&self, id: u64) -> bool {
        self.tasks.lock().unwrap().remove(&id).is_some()
    }

    pub fn reset_daily_tasks(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for task in tasks.values_mut() {
            if task.repeat_daily && task.completed {
                task.scheduled_time = task.scheduled_time + chrono::Duration::days(1);
                task.completed = false;
            }
        }
    }
}
