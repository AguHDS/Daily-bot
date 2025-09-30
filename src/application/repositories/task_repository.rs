use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use crate::application::domain::Task;

// In-memory repository for tasks
#[derive(Clone, Default)]
pub struct TaskRepository {
    tasks: Arc<Mutex<HashMap<u64, Task>>>, // id -> Task
    next_id: Arc<Mutex<u64>>, // for auto-incrementing ids
}

impl TaskRepository {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn add_task(&self, user_id: u64, message: String, scheduled_time: DateTime<Utc>, repeat_daily: bool) -> u64 {
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