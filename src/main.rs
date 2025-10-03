use dotenvy::dotenv;
mod application;
mod infrastructure;
use crate::infrastructure::discord_bot::bot::run_bot;
use crate::application::repositories::task_repository::TaskRepository;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize task repository and load tasks from storage
    let task_repo = TaskRepository::new();
    if let Err(e) = task_repo.load_all() {
        eprintln!("Could not load tasks from storage: {}", e);
    }

    // Start bot
    if let Err(e) = run_bot().await {
        eprintln!("Error running bot: {}", e);
    }
}
