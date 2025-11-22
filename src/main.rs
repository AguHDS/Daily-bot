use dotenvy::dotenv;
mod application;
mod domain;
mod infrastructure;
pub mod features;
mod utils;
use crate::infrastructure::discord_bot::bot::run_bot;
use tracing::{error};

#[tokio::main]
async fn main() {
    utils::setup_logging();
    dotenv().ok();

    if let Err(e) = run_bot().await {
        error!("Error running bot: {}", e);
    }
}
