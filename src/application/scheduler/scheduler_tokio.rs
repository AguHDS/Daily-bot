use std::sync::Arc;
use tokio::time::{sleep, Duration};
use chrono::Utc;
use serenity::prelude::Context;
use crate::application::repositories::task_repository::TaskRepository;

/// Scheduler loop que revisa tareas periódicamente y dispara recordatorios cuando corresponda
pub fn start_scheduler(ctx: Arc<Context>, repo: Arc<TaskRepository>) {
    // Se lanza en segundo plano
    tokio::spawn(async move {
        println!("[SCHEDULER] Scheduler started");

        loop {
            let now = Utc::now();
            let tasks = repo.list_tasks();

            for task in tasks {
                if task.scheduled_time <= now && !task.completed {
                    // Por ahora solo imprimimos el recordatorio
                    println!(
                        "[SCHEDULER] Reminder for user {}: {}",
                        task.user_id, task.message
                    );

                    // Marcamos la tarea como completada para que no se dispare de nuevo inmediatamente
                    repo.complete_task(task.id);
                }
            }

            // Espera 60 segundos antes de la siguiente revisión (puede ajustarse)
            sleep(Duration::from_secs(60)).await;
        }
    });
}
