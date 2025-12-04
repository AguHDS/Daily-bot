use crate::features::server_specific::config::voice_interaction_config::VoiceInteractionConfig;
use rand::Rng;
use serenity::all::{ChannelId, GuildId, UserId};
use serenity::http::Http;
use songbird::{Songbird, input::File};
use std::sync::Arc;
use std::{fs, path::Path};
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

pub struct VoiceInteractionService {
    config: VoiceInteractionConfig,
    http: Arc<Http>,
    songbird: Arc<Songbird>,
}

impl VoiceInteractionService {
    pub fn new(config: VoiceInteractionConfig, http: Arc<Http>, songbird: Arc<Songbird>) -> Self {
        Self {
            config,
            http,
            songbird,
        }
    }

    /// Check if user has permission to use voice interaction commands
    pub fn has_permission(&self, user_id: u64) -> bool {
        self.config.is_user_allowed(user_id)
    }

    /// Check if user has permission to request kicks
    pub fn can_kick(&self, user_id: u64) -> bool {
        self.config.can_user_kick(user_id)
    }

    /// Execute voice action (mute/disconnect) on target user
    pub async fn execute_voice_action(
        &self,
        guild_id: GuildId,
        target_user_id: u64,
        voice_channel_id: ChannelId,
        action: VoiceAction,
    ) -> Result<(), String> {
        let target_user_id = UserId::new(target_user_id);

        // 1. Join the voice channel
        self.join_voice_channel(guild_id, voice_channel_id).await?;

        // 2. Determine if we should play sound BEFORE the action
        //    Only for Mute and Disconnect, never for Unmute
        let should_play_sound = match action {
            VoiceAction::Mute | VoiceAction::Disconnect => self.should_play_sound(),
            VoiceAction::Unmute => false,
            VoiceAction::Kick => return Err("Kick action not supported in voice".to_string()),
        };

        // 3. FIRST: Play sound (if applicable)
        if should_play_sound {
            info!("Playing sound before {:?} action", action);
            match self.play_random_sound(guild_id).await {
                Ok(_) => {
                    info!("Sound played successfully before {:?}", action);
                    // Small pause after sound for dramatic effect
                    sleep(Duration::from_millis(500)).await;
                }
                Err(e) => {
                    warn!("Failed to play sound before action: {}", e);
                    // Continue even if sound fails
                }
            }
        } else {
            info!("No sound will be played for {:?} action", action);
        }

        // 4. THEN: Execute the action (mute/unmute/disconnect)
        match action {
            VoiceAction::Mute => {
                info!("Muting user {}", target_user_id);
                self.mute_user(guild_id, target_user_id).await?;
            }
            VoiceAction::Unmute => {
                info!("Unmuting user {}", target_user_id);
                self.unmute_user(guild_id, target_user_id).await?;
            }
            VoiceAction::Disconnect => {
                info!("Disconnecting user {}", target_user_id);
                self.disconnect_user(guild_id, target_user_id).await?;
            }
            VoiceAction::Kick => return Err("Kick action not supported in voice".to_string()),
        }

        // 5. Wait a few seconds before leaving
        info!("Waiting 3 seconds before leaving voice channel");
        sleep(Duration::from_secs(3)).await;
        self.leave_voice_channel(guild_id).await?;

        Ok(())
    }

    /// Determine if we should play a sound (50% chance)
    fn should_play_sound(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_bool(0.4) // 40% chance
    }

    /// Play a random sound from the sounds directory
    async fn play_random_sound(&self, guild_id: GuildId) -> Result<(), String> {
        // Try multiple locations
        let possible_paths = [
            Path::new("data/sounds"),                              // Producción
            Path::new("src/features/server_specific/data/sounds"), // Desarrollo
            Path::new("./sounds"),                                 // Raíz alternativa
        ];

        let sounds_dir = possible_paths
            .iter()
            .find(|path| path.exists())
            .ok_or_else(|| {
                let paths_str = possible_paths
                    .iter()
                    .map(|p| format!("{:?}", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("No sounds directory found. Tried: {}", paths_str)
            })?;

        info!("Using sounds directory: {:?}", sounds_dir);

        // Get all audio files from the directory
        let audio_extensions = ["mp3", "wav", "ogg", "flac", "m4a"];
        let mut sound_files = Vec::new();

        match fs::read_dir(sounds_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if let Some(ext_str) = ext.to_str() {
                                if audio_extensions.contains(&ext_str.to_lowercase().as_str()) {
                                    sound_files.push(path);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!(
                    "Failed to read sounds directory {:?}: {}",
                    sounds_dir, e
                ));
            }
        }

        // If no sound files found, return early
        if sound_files.is_empty() {
            return Err(format!(
                "No sound files found in directory: {:?}",
                sounds_dir
            ));
        }

        // Select a random sound file - usar el random_index ANTES de await para evitar problemas de Send
        let random_index = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..sound_files.len())
        }; // rng se destruye aquí
        
        let sound_file_path = sound_files[random_index].clone();

        info!("Playing random sound: {:?}", sound_file_path);

        // Verificar que el archivo existe antes de intentar cargarlo
        if !sound_file_path.exists() {
            return Err(format!("Sound file does not exist: {:?}", sound_file_path));
        }

        // Get the current voice call
        let handler = self
            .songbird
            .get(guild_id)
            .ok_or("Not in a voice channel".to_string())?;

        let mut handler_lock = handler.lock().await;

        // Stop any current audio
        handler_lock.stop();

        // Convert the sound file path to a format Songbird can use
        info!("Loading audio file: {:?}", sound_file_path);

        // Creat the input file (uses symphonya)
        let source = File::new(sound_file_path.clone());

        let track_handle = handler_lock.play_input(source.into());

        info!(
            "Started playing sound: {:?}",
            sound_file_path.file_name().unwrap_or_default()
        );

        // Wait for the audio to end (simplified version)
        // We use a fixed time of 10 seconds maximum
        let max_wait_time = Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < max_wait_time {
            // Check the track status
            if let Ok(info) = track_handle.get_info().await {
                if info.playing == songbird::tracks::PlayMode::Stop {
                    break;
                }
            }
            sleep(Duration::from_millis(100)).await;
        }


        // If the track is still playing after the maximum time, stop it
        if start_time.elapsed() >= max_wait_time {
            info!("Stopping sound after maximum wait time");
            track_handle
                .stop()
                .map_err(|e| format!("Failed to stop track: {}", e))?;
        }

        info!("Finished playing sound");
        Ok(())
    }

    async fn join_voice_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<(), String> {
        let _call = self
            .songbird
            .join(guild_id, channel_id)
            .await
            .map_err(|e| format!("Failed to join voice channel: {}", e))?;

        info!("Joined voice channel {} in guild {}", channel_id, guild_id);
        Ok(())
    }

    async fn leave_voice_channel(&self, guild_id: GuildId) -> Result<(), String> {
        self.songbird
            .leave(guild_id)
            .await
            .map_err(|e| format!("Failed to leave voice channel: {}", e))?;

        info!("Left voice channel in guild {}", guild_id);
        Ok(())
    }

    async fn mute_user(&self, guild_id: GuildId, user_id: UserId) -> Result<(), String> {
        self.http
            .edit_member(
                guild_id,
                user_id,
                &serde_json::json!({ "mute": true }),
                None,
            )
            .await
            .map_err(|e| format!("Failed to mute user: {}", e))?;
        info!("Muted user {} in guild {}", user_id, guild_id);
        Ok(())
    }

    async fn unmute_user(&self, guild_id: GuildId, user_id: UserId) -> Result<(), String> {
        self.http
            .edit_member(
                guild_id,
                user_id,
                &serde_json::json!({ "mute": false }),
                None,
            )
            .await
            .map_err(|e| format!("Failed to unmute user: {}", e))?;
        info!("Unmuted user {} in guild {}", user_id, guild_id);
        Ok(())
    }

    async fn disconnect_user(&self, guild_id: GuildId, user_id: UserId) -> Result<(), String> {
        self.http
            .edit_member(
                guild_id,
                user_id,
                &serde_json::json!({ "channel_id": serde_json::Value::Null }),
                None,
            )
            .await
            .map_err(|e| format!("Failed to disconnect user: {}", e))?;
        info!(
            "Disconnected user {} from voice in guild {}",
            user_id, guild_id
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VoiceAction {
    Mute,
    Unmute,
    Disconnect,
    Kick,
}
