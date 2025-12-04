use crate::features::server_specific::services::kick_service::KickService;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, warn};

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
    /// Only sends kick poll for one user per cycle at most
    async fn check_and_send_kick_polls(&self) {
        let targets_to_kick = self.service.get_targets_for_random_kick();

        if targets_to_kick.is_empty() {
            return;
        }

        // Since get_targets_for_random_kick() now returns at most one user,
        // we can safely process the first (and only) target
        if let Some(target) = targets_to_kick.first() {
            debug!("Found target for kick poll: {}", target.display_name);

            if let Err(e) = self.service.send_kick_poll_for_user(target.user_id).await {
                warn!(
                    "Failed to send kick poll for {}: {}",
                    target.display_name, e
                );
            }
            // No delay needed since only one poll is sent
        }
    }
}
