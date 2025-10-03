use crate::application::domain::task::{Recurrence, Task};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// In-memory repository for tasks with JSON persistence
#[derive(Clone, Default)]
pub struct TaskRepository {
    tasks: Arc<Mutex<HashMap<u64, Task>>>, // id -> Task
    next_id: Arc<Mutex<u64>>,              // auto-increment IDs
}

impl TaskRepository {
    const FILE_PATH: &'static str = "tasks.json";

    // Save all tasks to JSON file
    pub fn save_all(&self) -> std::io::Result<()> {
        let all_tasks: Vec<Task> = {
            let tasks = self.tasks.lock().unwrap();
            tasks.values().cloned().collect()
        };
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

    // Create new repository and load tasks from JSON if any
    pub fn new() -> Self {
        let repo = Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        };
        let _ = repo.load_all(); // ignore errors on startup
        repo
    }

    // Add a task and save to JSON
    pub fn add_task(
        &self,
        user_id: u64,
        message: String,
        scheduled_time: Option<DateTime<Utc>>,
        recurrence: Option<Recurrence>,
    ) -> u64 {
        let id = {
            let mut id_lock = self.next_id.lock().unwrap();
            let id = *id_lock;
            *id_lock += 1;
            id
        };

        let task = Task::new(id, user_id, message, scheduled_time, recurrence);

        {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.insert(id, task);
        }

        let _ = self.save_all(); // Guardar fuera del lock
        id
    }

    // List all tasks
    pub fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.values().cloned().collect()
    }

    // Complete a task and save to JSON
    pub fn complete_task(&self, id: u64) -> bool {
        let updated = {
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(&id) {
                task.completed = true;
                true
            } else {
                false
            }
        };

        if updated {
            let _ = self.save_all();
        }

        updated
    }

    // Remove a task and save to JSON
    pub fn remove_task(&self, id: u64) -> bool {
        let removed = {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.remove(&id).is_some()
        };

        if removed {
            let _ = self.save_all();
        }

        removed
    }

    // Remove all tasks for a specific user and save to JSON
    pub fn remove_all_by_user(&self, user_id: u64) -> usize {
        let mut tasks = self.tasks.lock().unwrap();
        let before = tasks.len();
        tasks.retain(|_, task| task.user_id != user_id);
        before - tasks.len()
    }
}
