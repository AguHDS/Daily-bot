use crate::application::domain::task::{Recurrence, Task};
use crate::application::repositories::task_repository::TaskRepository;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// implementation of TaskRepository that stores tasks in JSON
#[derive(Clone, Default)]
pub struct JsonTaskRepository {
    tasks: Arc<Mutex<HashMap<u64, Task>>>,
    next_id: Arc<Mutex<u64>>,
    file_path: String,
}

impl JsonTaskRepository {
    pub fn new(file_path: &str) -> Self {
        let repo = Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            file_path: file_path.to_string(),
        };
        let _ = repo.load_all();
        repo
    }

    fn save_all(&self) -> std::io::Result<()> {
        let all_tasks: Vec<Task> = {
            let tasks = self.tasks.lock().unwrap();
            tasks.values().cloned().collect()
        };
        crate::application::repositories::json_storage::save_tasks(&all_tasks, &self.file_path)
    }

    fn load_all(&self) -> std::io::Result<()> {
        let loaded_tasks = crate::application::repositories::json_storage::load_tasks(&self.file_path)?;
        let mut tasks = self.tasks.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        for task in loaded_tasks {
            if task.id >= *next_id { // desreference next_id to get current id
                *next_id = task.id + 1;
            }
            tasks.insert(task.id, task);
        }
        Ok(())
    }
}

impl TaskRepository for JsonTaskRepository {
    fn add_task(&self, mut task: Task) -> Result<u64, String> {
        let id = {
            let mut id_lock = self.next_id.lock().unwrap();
            // dereference id_lock to get current id
            let id = *id_lock;
            *id_lock += 1;
            id
        };
        task.id = id;

        {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.insert(id, task);
        }

        let _ = self.save_all();
        Ok(id)
    }

    fn edit_task(
        &self,
        task_id: u64,
        new_message: Option<String>,
        new_scheduled_time: Option<DateTime<Utc>>,
        new_recurrence: Option<Recurrence>,
    ) -> Result<Task, String> {
        let updated_task = {
            let mut tasks = self.tasks.lock().unwrap();
            let task = tasks
                .get_mut(&task_id)
                .ok_or_else(|| format!("Couldn't find task with ID {}", task_id))?;

            if let Some(msg) = new_message {
                if msg.trim().is_empty() {
                    return Err("Task title can not be empty".to_string());
                }
                task.message = msg;
            }
            if let Some(new_time) = new_scheduled_time {
                task.scheduled_time = Some(new_time);
            }
            if let Some(recur) = new_recurrence {
                task.recurrence = Some(recur);
            }

            task.clone()
        };

        let _ = self.save_all();
        Ok(updated_task)
    }

    fn remove_task(&self, task_id: u64) -> bool {
        let removed = {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.remove(&task_id).is_some()
        };
        if removed {
            let _ = self.save_all();
        }
        removed
    }

    fn remove_all_by_user(&self, user_id: u64) -> usize {
        let removed_count = {
            let mut tasks = self.tasks.lock().unwrap();
            let before = tasks.len();
            tasks.retain(|_, task| task.user_id != user_id);
            before - tasks.len()
        };
        if removed_count > 0 {
            let _ = self.save_all();
        }
        removed_count
    }

    fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.values().cloned().collect()
    }
}
