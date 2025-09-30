use dotenvy::dotenv;
mod application;
mod infrastructure;
use crate::infrastructure::discord_bot::run_bot;

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(e) = run_bot().await {
        eprintln!("Error running bot: {}", e);
    }
}
