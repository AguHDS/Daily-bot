use dotenvy::dotenv;
use std::env;
mod application;
mod infrastructure;
use crate::infrastructure::discord_bot::run_bot;

#[tokio::main]
async fn main() {
    dotenv().ok();

    match env::var("DISCORD_TOKEN") {
        Ok(token) => println!("Token cargado correctamente: {}...", &token[0..5]),
        Err(err) => eprintln!("No se pudo cargar DISCORD_TOKEN: {}", err),
    }

    if let Err(e) = run_bot().await {
        eprintln!("Error running bot: {}", e);
    }
}
