use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn setup_logging() {
    // Create a subscriber that logs to stdout
    let subscriber = FmtSubscriber::builder()
        // Set the maximum log level (adjust for production)
        .with_max_level(Level::INFO)
        // Complete the builder
        .finish();

    // Set the global default subscriber
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");
}