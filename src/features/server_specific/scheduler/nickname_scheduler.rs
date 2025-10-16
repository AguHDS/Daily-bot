use crate::features::server_specific::services::nickname_changer::NicknameChangerService;
use chrono::Local;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

pub struct NicknameScheduler {
    service: Arc<NicknameChangerService>,
}

impl NicknameScheduler {
    pub fn new(service: Arc<NicknameChangerService>) -> Self {
        Self { service }
    }

    /// Starts the nickname change scheduler
    pub async fn start(self) {
        tokio::spawn(async move {
            loop {
                self.check_and_change_nicknames().await;
                // Check every minute for schedule matches
                sleep(Duration::from_secs(60)).await;
            }
        });
    }

    /// Checks current time and changes nicknames for scheduled targets
    async fn check_and_change_nicknames(&self) {
        let current_time = Local::now().format("%H:%M:%S").to_string();
        log::info!(
            "[SCHEDULER] Checking for nickname changes at {}",
            current_time
        );

        // Get targets that should have their nicknames changed right now (Argentina time)
        let targets_to_change = self.service.get_scheduled_targets_for_current_time();

        log::info!(
            "[SCHEDULER] Found {} targets to change",
            targets_to_change.len()
        );

        if targets_to_change.is_empty() {
            log::debug!("[SCHEDULER] No nickname changes scheduled for current time");
            return;
        }

        for target in targets_to_change {
            log::info!(
                "[SCHEDULER] Attempting to change nickname for {} (user_id: {})",
                target.display_name,
                target.user_id
            );

            match self.service.change_nickname_for_user(target.user_id).await {
                Ok(result) => {
                    log::info!("[SCHEDULER] Success: {}", result);
                }
                Err(e) => {
                    log::warn!(
                        "[SCHEDULER] Failed to change nickname for {}: {}",
                        target.display_name,
                        e
                    );
                }
            }

            // Small delay between changes to avoid rate limiting
            sleep(Duration::from_secs(2)).await;
        }
    }
}
