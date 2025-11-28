use crate::application::commands::utils::date_format::{
    get_inferred_date_format_info, get_user_date_format_info,
};
use crate::application::services::timezone_service::TimezoneService;
use serenity::builder::CreateEmbedFooter;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuOption,
};
use serenity::model::application::CommandInteraction;
use serenity::model::colour::Colour;
use serenity::prelude::*;
use std::sync::Arc;
use tracing::error;

pub fn register_timezone_command() -> serenity::builder::CreateCommand {
    serenity::builder::CreateCommand::new("timezone")
        .description("Set your time zone for tasks")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                serenity::model::application::CommandOptionType::String,
                "location",
                "Your country, city or state (e.g. Argentina, New York, Madrid)",
            )
            .required(true),
        )
}

pub async fn run_timezone_command(
    ctx: &Context,
    command: &CommandInteraction,
    timezone_service: &Arc<TimezoneService>,
) {
    let user_id = command.user.id.get();
    let location = match crate::application::commands::utils::get_string::get_string_option(
        &command.data.options,
        0,
    ) {
        Some(loc) => loc,
        None => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ You must provide a location (country, city or state)")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // find timezones that match the location
    let search_results = timezone_service.search_timezones(&location);

    if search_results.is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "❌ No time zones found for '{}'. Try a more specific name",
                            location
                        ))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // if there are multiple results, show selection
    if search_results.len() > 1 {
        show_timezone_selection(ctx, command, user_id, search_results, timezone_service).await;
    } else {
        // if there is only one result, show confirmation directly
        let timezone_info = search_results[0];
        show_timezone_confirmation(ctx, command, user_id, timezone_info, timezone_service).await;
    }
}

// Show timezone selection menu when there are multiple results
async fn show_timezone_selection(
    ctx: &Context,
    command: &CommandInteraction,
    _user_id: u64,
    timezones: Vec<&crate::infrastructure::timezone::timezone_manager::TimezoneInfo>,
    timezone_service: &Arc<TimezoneService>,
) {
    let mut options = Vec::new();

    for (_index, tz_info) in timezones.iter().enumerate() {
        let current_time = match timezone_service.get_current_time_for_timezone(&tz_info.utc[0]) {
            Ok(time) => time,
            Err(_) => "Error al obtener hora".to_string(),
        };

        let label = if tz_info.utc.len() == 1 {
            format!("{} - {}", tz_info.text, current_time)
        } else {
            format!("{} - {}", tz_info.value, current_time)
        };

        let mut description = if tz_info.utc.len() == 1 {
            tz_info.utc[0].clone()
        } else {
            tz_info.utc[0..2].join(", ") + "..."
        };

        if description.len() > 50 {
            description.truncate(50);
        }

        let timezone_id = &tz_info.utc[0];
        options
            .push(CreateSelectMenuOption::new(label, timezone_id.clone()).description(description));
    }

    let select_menu = CreateSelectMenu::new(
        "timezone_select",
        serenity::builder::CreateSelectMenuKind::String { options },
    )
    .placeholder("Select your timezone")
    .min_values(1)
    .max_values(1);

    let action_row = CreateActionRow::SelectMenu(select_menu);

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("🔍 **Select your timezone:**")
                    .components(vec![action_row])
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Manage /timezone confirmation when there is a single result
async fn show_timezone_confirmation(
    ctx: &Context,
    command: &CommandInteraction,
    _user_id: u64,
    timezone_info: &crate::infrastructure::timezone::timezone_manager::TimezoneInfo,
    timezone_service: &Arc<TimezoneService>,
) {
    let timezone_id = &timezone_info.utc[0];
    let current_time = match timezone_service.get_current_time_for_timezone(timezone_id) {
        Ok(time) => time,
        Err(e) => {
            error!("Error getting current time: {:?}", e);
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Error obtaining current time")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Use utility function to get date format info
    let (_inferred_format, format_description) =
        get_inferred_date_format_info(timezone_service, timezone_id);

    let embed = CreateEmbed::new()
        .title("🕐 Timezone confirmation")
        .description(format!(
            "**Selected timezone:** {}\n**Current time:** `{}`\n**Date format:** {}\n\nIs this your correct local time?",
            timezone_info.text, current_time, format_description
        ))
        .colour(Colour::DARK_GREEN)
        .footer(CreateEmbedFooter::new(
            "Date format will be automatically set based on your location. You can change it later if needed.",
        ));

    let accept_button = CreateButton::new(format!("timezone_confirm:{}", timezone_id))
        .label("✅ Yes, it's correct")
        .style(serenity::all::ButtonStyle::Success);

    let cancel_button = CreateButton::new("timezone_cancel")
        .label("❌ Cancel")
        .style(serenity::all::ButtonStyle::Danger);

    let action_row = CreateActionRow::Buttons(vec![accept_button, cancel_button]);

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![action_row])
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Manage /timezone when there is multiple results
pub async fn handle_timezone_select(
    ctx: &Context,
    interaction: &serenity::model::application::ComponentInteraction,
    timezone_service: &Arc<TimezoneService>,
) {
    // get the selected timezone_id
    let timezone_id = match &interaction.data.kind {
        serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
            if let Some(first_value) = values.first() {
                first_value.clone()
            } else {
                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("❌ No selection was found")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        }
        _ => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Interaction type unvalid")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    let timezone_info = match timezone_service.get_timezone_info(&timezone_id) {
        Some(info) => info,
        None => {
            // Fallback: intentar con search_timezones - CORREGIDO EL LIFETIME
            let search_results = timezone_service.search_timezones(&timezone_id);
            if search_results.is_empty() {
                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("❌ Timezone not found")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
            search_results[0]
        }
    };

    // show confirmation for selected timezone
    let current_time = match timezone_service.get_current_time_for_timezone(&timezone_id) {
        Ok(time) => time,
        Err(e) => {
            error!("Error getting current time: {:?}", e);
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Error obtaining current time")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Use utility function to get date format info
    let (_inferred_format, format_description) =
        get_inferred_date_format_info(timezone_service, &timezone_id);

    let embed = CreateEmbed::new()
        .title("🕐 Timezone confirmation")
        .description(format!(
            "**Selected timezone:** {}\n**Current time:** `{}`\n**Date format:** {}\n\nIs this your correct local time?",
            timezone_info.text, current_time, format_description
        ))
        .colour(Colour::DARK_GREEN)
        .footer(CreateEmbedFooter::new(
            "Date format will be automatically set based on your location. You can change it later if needed.",
        ));

    let accept_button = CreateButton::new(format!("timezone_confirm:{}", timezone_id))
        .label("✅ Yes, it's correct")
        .style(serenity::all::ButtonStyle::Success);

    let cancel_button = CreateButton::new("timezone_cancel")
        .label("❌ Cancel")
        .style(serenity::all::ButtonStyle::Danger);

    let action_row = CreateActionRow::Buttons(vec![accept_button, cancel_button]);

    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![action_row])
                    .ephemeral(true),
            ),
        )
        .await;
}

pub async fn handle_timezone_confirm(
    ctx: &Context,
    interaction: &serenity::model::application::ComponentInteraction,
    timezone_id: &str,
    timezone_service: &Arc<TimezoneService>,
) {
    let user_id = interaction.user.id.get();

    match timezone_service
        .set_user_timezone(user_id, timezone_id)
        .await
    {
        Ok(()) => {
            let current_time = match timezone_service.get_current_time_for_user(user_id).await {
                Ok(time) => time,
                Err(_) => "Error obtaining hour".to_string(),
            };

            // Use utility function to get user's date format info
            let date_format_info = get_user_date_format_info(timezone_service, user_id).await;

            let embed = CreateEmbed::new()
                .title("✅ Timezone set up correctly!")
                .description(format!(
                    "Timezone configured successfully for {}!",
                    interaction.user.mention()
                ))
                .field("Timezone", format!("`{}`", timezone_id), true)
                .field("Current Time", format!("`{}`", current_time), true)
                .field("Date Format", date_format_info, false)
                .color(serenity::model::colour::Colour::DARK_GREEN)
                .footer(CreateEmbedFooter::new(
                    "When creating tasks, the date field will now show the format familiar to your region",
                ));

            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .ephemeral(false),
                    ),
                )
                .await;
        }
        Err(e) => {
            error!("Error setting timezone: {:?}", e);
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ Error setting time zone")
                            .ephemeral(true),
                    ),
                )
                .await;
        }
    }
}

pub async fn handle_timezone_cancel(
    ctx: &Context,
    interaction: &serenity::model::application::ComponentInteraction,
) {
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("❌ Time zone setting canceled")
                    .ephemeral(false),
            ),
        )
        .await;
}
