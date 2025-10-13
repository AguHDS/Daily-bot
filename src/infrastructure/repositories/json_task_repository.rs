use crate::domain::entities::task::{NotificationMethod, Recurrence, Task};
use crate::domain::repositories::TaskRepository;
use crate::infrastructure::repositories::json_storage;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Implementation of TaskRepository that stores tasks in JSON
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
        json_storage::save_tasks(&all_tasks, &self.file_path)
    }

    fn load_all(&self) -> std::io::Result<()> {
        let loaded_tasks = json_storage::load_tasks(&self.file_path)?;
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
}

impl TaskRepository for JsonTaskRepository {
    fn add_task(&self, mut task: Task) -> Result<u64, String> {
        let id = {
            let mut id_lock = self.next_id.lock().unwrap();
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
        new_title: Option<String>,
        new_description: Option<String>,
        new_scheduled_time: Option<DateTime<Utc>>,
        new_recurrence: Option<Recurrence>,
        new_notification_method: Option<NotificationMethod>,
    ) -> Result<Task, String> {
        let updated_task = {
            let mut tasks = self.tasks.lock().unwrap();
            let task = tasks
                .get_mut(&task_id)
                .ok_or_else(|| format!("Couldn't find task with ID {}", task_id))?;

            // updates
            if let Some(title) = new_title {
                if title.trim().is_empty() {
                    return Err("Task title cannot be empty".to_string());
                }
                task.title = title;
            }

            if let Some(description) = new_description {
                task.description = Some(description);
            }
            if let Some(new_time) = new_scheduled_time {
                task.scheduled_time = Some(new_time);
            }
            if let Some(recur) = new_recurrence {
                task.recurrence = Some(recur);
            }
            if let Some(notif) = new_notification_method {
                task.notification_method = notif;
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

    fn update_task_time(&self, task_id: u64, new_time: DateTime<Utc>) -> Result<(), String> {
        {
            let mut tasks = self.tasks.lock().unwrap();
            let task = tasks
                .get_mut(&task_id)
                .ok_or_else(|| format!("Couldn't find task with ID {}", task_id))?;
            task.scheduled_time = Some(new_time);
        }
        self.save_all().map_err(|e| e.to_string())?;
        Ok(())
    }
}
