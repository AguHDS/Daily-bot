use dotenvy::dotenv;
mod application;
mod domain;
mod infrastructure;
use crate::infrastructure::discord_bot::bot::run_bot;

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(e) = run_bot().await {
        eprintln!("Error running bot: {}", e);
    }
}
