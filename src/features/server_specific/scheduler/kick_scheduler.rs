use crate::features::server_specific::services::kick_service::KickService;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

pub struct KickScheduler {
    service: Arc<KickService>,
}

impl KickScheduler {
    pub fn new(service: Arc<KickService>) -> Self {
        Self { service }
    }

    /// Starts the kick poll scheduler with random timing
    pub async fn start(self) {
        let check_interval = self.get_check_interval();
        info!(
            "Starting kick scheduler with {} minute interval",
            check_interval.as_secs() / 60
        );

        tokio::spawn(async move {
            loop {
                self.check_and_send_kick_polls().await;
                sleep(check_interval).await;
            }
        });
    }

    /// Gets the check interval from the configuration
    fn get_check_interval(&self) -> Duration {
        Duration::from_secs(
            self.service
                .kick_config
                .random_config
                .check_interval_minutes as u64
                * 60,
        )
    }

    /// Checks and sends kick polls based on random probability
    async fn check_and_send_kick_polls(&self) {
        let targets_to_kick = self.service.get_targets_for_random_kick();

        if !targets_to_kick.is_empty() {
            debug!("Found {} targets for kick polls", targets_to_kick.len());
        }

        for target in targets_to_kick {
            if let Err(e) = self.service.send_kick_poll_for_user(target.user_id).await {
                warn!(
                    "Failed to send kick poll for {}: {}",
                    target.display_name, e
                );
            }

            // Small delay between polls
            sleep(Duration::from_secs(2)).await;
        }
    }
}
