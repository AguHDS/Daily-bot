use crate::features::server_specific::services::nickname_changer::NicknameChangerService;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, warn};

pub struct NicknameScheduler {
    service: Arc<NicknameChangerService>,
}

impl NicknameScheduler {
    pub fn new(service: Arc<NicknameChangerService>) -> Self {
        Self { service }
    }

    /// Starts the nickname change scheduler with random timing
    pub async fn start(self) {
        let check_interval = self.get_check_interval();

        tokio::spawn(async move {
            loop {
                self.check_and_change_nicknames().await;
                sleep(check_interval).await;
            }
        });
    }

    /// Gets the check interval from the configuration
    fn get_check_interval(&self) -> Duration {
        Duration::from_secs(
            self.service
                .nickname_config
                .random_config
                .check_interval_minutes as u64
                * 60,
        )
    }

    /// Checks and changes nicknames based on random probability
    /// Only changes nickname for one user per cycle at most
    async fn check_and_change_nicknames(&self) {
        let targets_to_change = self.service.get_targets_for_random_change();

        if targets_to_change.is_empty() {
            return;
        }

        // Since get_targets_for_random_change() now returns at most one user,
        // we can safely process the first (and only) target
        if let Some(target) = targets_to_change.first() {
            debug!("Found target for nickname change: {}", target.display_name);

            if let Err(e) = self.service.change_nickname_for_user(target.user_id).await {
                warn!(
                    "Failed to change nickname for {}: {}",
                    target.display_name, e
                );
            }
        }
    }
}
