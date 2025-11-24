use crate::features::server_specific::config::kick_config::KickTargetUser;
use crate::features::server_specific::services::kick_service::KickService;
use crate::features::server_specific::utils::extract_username_from_kick_message;

use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
    Http, UserId,
};
use serenity::prelude::*;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ServerInteractionHandler {
    pub kick_service: Option<Arc<KickService>>,
}

impl ServerInteractionHandler {
    pub fn new(kick_service: Option<Arc<KickService>>) -> Self {
        Self { kick_service }
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
                    let response = format!("No se pudo encontrar al usuario: {}", server_name);
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
