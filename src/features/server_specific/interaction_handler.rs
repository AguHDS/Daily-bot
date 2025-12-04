use crate::features::server_specific::config::kick_config::KickTargetUser;
use crate::features::server_specific::services::{
    alias_service::AliasService,
    kick_service::KickService,
    voice_interaction_service::VoiceInteractionService,
};
use crate::features::server_specific::utils::extract_username_from_kick_message;

use serenity::all::{
    ChannelId, ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    GuildId, Http, Message, UserId,
};
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ServerInteractionHandler {
    pub kick_service: Option<Arc<KickService>>,
    pub voice_interaction_service: Option<Arc<VoiceInteractionService>>,
    pub alias_service: Option<Arc<AliasService>>,
}

impl ServerInteractionHandler {
    // Protected user ID (ponyrosa) - cannot be muted, kicked, disconnected, etc.
    const PROTECTED_USER_ID: u64 = 300869447475003393;

    pub fn new(
        kick_service: Option<Arc<KickService>>,
        voice_interaction_service: Option<Arc<VoiceInteractionService>>,
        alias_service: Option<Arc<AliasService>>,
    ) -> Self {
        Self {
            kick_service,
            voice_interaction_service,
            alias_service,
        }
    }

    /// Handles all server-specific button interactions
    pub async fn handle_button_interaction(&self, ctx: &Context, component: &ComponentInteraction) {
        let custom_id = &component.data.custom_id;

        match custom_id.as_str() {
            "kick_yes" => {
                self.handle_kick_decision(ctx, component, true).await;
            }
            "kick_no" => {
                self.handle_kick_decision(ctx, component, false).await;
            }
            _ => {
                debug!("Unknown button interaction: {}", custom_id);
            }
        }
    }

    /// Handles message interactions (when bot is mentioned)
    pub async fn handle_message_interaction(&self, ctx: &Context, message: &Message) {
        if !message.mentions_me(&ctx.http).await.unwrap_or(false) {
            return;
        }

        let content = message.content.to_lowercase();
        let author_id = message.author.id.get();
        let bot_id = ctx.cache.current_user().id.get();
        let guild_id = message.guild_id.unwrap();

        // Check permission for voice interaction commands
        if let Some(voice_service) = &self.voice_interaction_service {
            // Verify if the command is "desmuteame" for self-unmute
            let is_desmuteame_self = self.is_desmuteame_self_command(&content);

            if !is_desmuteame_self && !voice_service.has_permission(author_id) {
                let _ = message.channel_id.say(&ctx.http, "Callate puta").await;
                return;
            }

            // Parse voice interaction commands
            if let Some((action, target_user)) = self
                .parse_voice_command(&content, &message.mentions, author_id, bot_id, message)
                .await
            {
                if let Some(target_id) = target_user {
                    // Check if target is protected user
                    if target_id == Self::PROTECTED_USER_ID
                        && !(target_id == author_id
                            && matches!(
                                action,
                                crate::features::server_specific::services::voice_interaction_service::VoiceAction::Unmute
                            ))
                    {
                        let _ = message.channel_id.say(&ctx.http, "Callate puta").await;
                        return;
                    }

                    // Obtain the voice channel of the target user
                    match self
                        .get_user_voice_channel_from_ctx(ctx, guild_id, target_id)
                        .await
                    {
                        Some(voice_channel_id) => {
                            let _ = message.channel_id.say(&ctx.http, "Bueno").await;

                            let voice_service_clone = voice_service.clone();
                            let ctx_http = ctx.http.clone();
                            let message_channel_id = message.channel_id;

                            tokio::spawn(async move {
                                match voice_service_clone
                                    .execute_voice_action(
                                        guild_id,
                                        target_id,
                                        voice_channel_id,
                                        action,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        debug!("Voice action completed successfully");

                                        if let crate::features::server_specific::services::voice_interaction_service::VoiceAction::Unmute = action {
                                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to execute voice action in background: {}",
                                            e
                                        );
                                        let _ = message_channel_id
                                            .say(&ctx_http, format!("Error en la acción: {}", e))
                                            .await;
                                    }
                                }
                            });
                        }
                        None => {
                            let _ = message
                                .channel_id
                                .say(&ctx.http, "Pero no está en ningún canal puta")
                                .await;
                        }
                    }
                } else {
                    let _ = message
                        .channel_id
                        .say(&ctx.http, "Callate puta")
                        .await;
                }
                return;
            }
        }

        // Handle "kick" commands (server kick, not voice)
        if let Some(kick_service) = &self.kick_service {
            if self.is_kick_command(&content) {
                let target_user = if let Some(mentioned_user) = message.mentions.get(1) {
                    Some(mentioned_user.id.get())
                } else if let Some(alias_service) = &self.alias_service {
                    alias_service.extract_user_id_from_content(&content).await
                } else {
                    None
                };

                if let Some(target_id) = target_user {
                    // Check if target is protected user
                    if target_id == Self::PROTECTED_USER_ID {
                        let _ = message.channel_id.say(&ctx.http, "Callate puta").await;
                        return;
                    }

                    // Verificar permiso de kick
                    if let Some(voice_service) = &self.voice_interaction_service {
                        if !voice_service.can_kick(author_id) {
                            let _ = message.channel_id.say(&ctx.http, "Callate puta").await;
                            return;
                        }
                    }

                    let _ = message.channel_id.say(&ctx.http, "bueno").await;

                    let kick_service_clone = kick_service.clone();
                    let ctx_http = ctx.http.clone();
                    let message_channel_id = message.channel_id;

                    tokio::spawn(async move {
                        match kick_service_clone.execute_kick(target_id).await {
                            Ok(_) => {
                                debug!("Kick action completed successfully");
                            }
                            Err(e) => {
                                error!("Failed to kick user in background: {}", e);
                                let _ = message_channel_id
                                    .say(&ctx_http, format!("Error al kickear: {}", e))
                                    .await;
                            }
                        }
                    });
                } else {
                    let _ = message
                        .channel_id
                        .say(&ctx.http, "Menciona al usuario o usa un alias conocido")
                        .await;
                }
                return;
            }
        }

        // If no command matched, send default response
        let _ = message.channel_id.say(&ctx.http, "Que homosexual").await;
    }

    async fn get_user_voice_channel_from_ctx(
        &self,
        ctx: &Context,
        guild_id: GuildId,
        user_id: u64,
    ) -> Option<ChannelId> {
        if let Some(guild) = ctx.cache.guild(guild_id) {
            guild
                .voice_states
                .get(&UserId::new(user_id))
                .and_then(|state| state.channel_id)
        } else {
            None
        }
    }

    /// Parse voice interaction commands from message content
    async fn parse_voice_command(
        &self,
        content: &str,
        mentions: &[serenity::all::User],
        author_id: u64,
        bot_id: u64,
        _message: &Message,
    ) -> Option<(
        crate::features::server_specific::services::voice_interaction_service::VoiceAction,
        Option<u64>,
    )> {
        let content = content.trim();

        // Verify if it's a self-unmute command
        if self.is_desmuteame_self_command(content) {
            // When the user says "desmuteame" without mentioning anyone else,
            return Some((
                crate::features::server_specific::services::voice_interaction_service::VoiceAction::Unmute,
                Some(author_id),
            ));
        }

        // Try to find target user from mentions or aliases
        let target_user = if let Some(mentioned_user) = mentions
            .iter()
            .find(|mentioned| {
                let mentioned_id = mentioned.id.get();
                mentioned_id != author_id && mentioned_id != bot_id
            })
        {
            Some(mentioned_user.id.get())
        } else if let Some(alias_service) = &self.alias_service {
            alias_service.extract_user_id_from_content(content).await
        } else {
            None
        };

        // Mute commands
        if self.is_mute_command(content) {
            return Some((
            crate::features::server_specific::services::voice_interaction_service::VoiceAction::Mute,
            target_user,
        ));
        }

        // Unmute commands (cuando se menciona a otro usuario)
        if self.is_unmute_command(content) {
            return Some((
            crate::features::server_specific::services::voice_interaction_service::VoiceAction::Unmute,
            target_user,
        ));
        }

        // Disconnect commands
        if self.is_disconnect_command(content) {
            return Some((
            crate::features::server_specific::services::voice_interaction_service::VoiceAction::Disconnect,
            target_user,
        ));
        }

        None
    }

    /// Check if the content contains the specific "desmuteame" command for self-unmute
    fn is_desmuteame_self_command(&self, content: &str) -> bool {
        let desmuteame_patterns = [
            "desmuteame",
            "desmutéame",
            "des-muteame",
            "des muteame",
            "desmutearme",
            "desmutea me",
        ];

        desmuteame_patterns
            .iter()
            .any(|&pattern| content.contains(pattern))
    }

    fn is_mute_command(&self, content: &str) -> bool {
        let mute_keywords = [
            "mutea",
            "muteame",
            "muteate",
            "mutealo",
            "mutear",
            "muteamelo",
            "silenciar",
            "silencialo",
            "silencia",
            "silenciame",
            "silenciamelo",
            "calla",
            "callar",
            "callamelo",
            "callalo",
            "callame",
        ];

        let has_mute_keyword = mute_keywords
            .iter()
            .any(|&keyword| content.contains(keyword));

        let has_unmute_prefix = content.contains("desmute")
            || content.contains("desilencia")
            || content.contains("descalla");

        has_mute_keyword && !has_unmute_prefix
    }

    /// Check if the content contains any unmute-related command
    fn is_unmute_command(&self, content: &str) -> bool {
        let unmute_keywords = [
            "desmutea",
            "desmuteate",
            "desmuteame",
            "desmutealo",
            "desmutear",
            "desilencia",
            "desilenciar",
            "desilenciame",
            "desilencialo",
            "descalla",
            "descallar",
            "descallame",
            "descallalo",
        ];

        unmute_keywords
            .iter()
            .any(|&keyword| content.contains(keyword))
    }

    /// Check if the content contains any disconnect-related command
    fn is_disconnect_command(&self, content: &str) -> bool {
        let disconnect_keywords = [
            "desconecta",
            "desconectar",
            "desconectamelo",
            "desconectalo",
            "desconectame",
            "sacamelo",
            "sacar",
            "saca",
            "echar",
            "echamelo",
            "echame",
        ];

        disconnect_keywords
            .iter()
            .any(|&keyword| content.contains(keyword))
    }

    /// Check if the content contains any kick-related command
    fn is_kick_command(&self, content: &str) -> bool {
        let kick_keywords = ["kickealo", "kickeamelo", "kickea", "kickear"];

        kick_keywords
            .iter()
            .any(|&keyword| content.contains(keyword))
    }

    async fn handle_kick_decision(
        &self,
        ctx: &Context,
        component: &ComponentInteraction,
        approved: bool,
    ) {
        let original_message = component.message.content.clone();

        if original_message.is_empty() {
            error!("No content in kick poll message");
            return;
        }

        // Extract username from message
        let server_name = extract_username_from_kick_message(&original_message);

        if let (Some(kick_service), Some(server_name)) = (&self.kick_service, server_name) {
            if approved {
                // Find user ID by server name
                if let Some(target) = self
                    .find_target_by_server_name(&server_name, &ctx.http, kick_service)
                    .await
                {
                    // Check if target is protected user
                    if target.user_id == Self::PROTECTED_USER_ID {
                        let response = "Callate puta".to_string();
                        let _ = component
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::UpdateMessage(
                                    CreateInteractionResponseMessage::new()
                                        .content(response)
                                        .components(vec![]),
                                ),
                            )
                            .await;
                        return;
                    }

                    match kick_service.execute_kick(target.user_id).await {
                        Ok(_) => {
                            let response = format!("{} kickeado.", server_name);
                            let _ = component
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(response)
                                            .components(vec![]),
                                    ),
                                )
                                .await;
                        }
                        Err(e) => {
                            let response = format!("Error al kickear a {}: {}", server_name, e);
                            let _ = component
                                .create_response(
                                    &ctx.http,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .content(response)
                                            .components(vec![]),
                                    ),
                                )
                                .await;
                            error!("Failed to kick user {}: {}", server_name, e);
                        }
                    }
                } else {
                    let response = format!("No encontré al usuario: {}", server_name);
                    let _ = component
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .content(response)
                                    .components(vec![]),
                            ),
                        )
                        .await;
                }
            } else {
                let response = format!("bueno...");
                let _ = component
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content(response)
                                .components(vec![]),
                        ),
                    )
                    .await;
            }
        } else {
            error!("Kick service not available or username not found");
        }
    }

    /// Find target by server name
    async fn find_target_by_server_name(
        &self,
        server_name: &str,
        http: &Http,
        kick_service: &KickService,
    ) -> Option<KickTargetUser> {
        let guild_id = GuildId::new(kick_service.server_config.server_id);

        for target in &kick_service.kick_config.targets {
            let user_id = UserId::new(target.user_id);

            match guild_id.member(http, user_id).await {
                Ok(member) => {
                    let target_server_name = member
                        .nick
                        .clone()
                        .unwrap_or_else(|| member.user.name.clone());
                    if target_server_name == server_name {
                        return Some(target.clone());
                    }
                }
                Err(_) => {
                    if target.display_name == server_name {
                        return Some(target.clone());
                    }
                }
            }
        }
        None
    }
}
