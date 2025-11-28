use crate::application::commands::utils::get_string_option;
use crate::application::services::TaskOrchestrator;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::entities::task::NotificationMethod;
use serenity::{
    all::{
        ActionRowComponent, CommandInteraction, CommandOptionType, CreateCommand,
        CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
        InputTextStyle, ModalInteraction,
    },
    builder::{CreateActionRow, CreateInputText, CreateModal},
    prelude::*,
};
use std::sync::Arc;
use tracing::error;

pub fn register_add_task_command() -> CreateCommand {
    CreateCommand::new("add_task")
        .description("Add a new task")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "notification_method",
                "How to notify you when the task is due",
            )
            .add_string_choice("Direct Message", "DM")
            .add_string_choice("Channel Notification", "Channel")
            .add_string_choice("Both DM and Channel", "Both")
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "task_type",
                "Task type: single or weekly",
            )
            .add_string_choice("Single (specific date/time)", "single")
            .add_string_choice("Weekly (repeats on specific days)", "weekly")
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "Channel for notifications (required for Channel/Both)",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "mention",
                "@mention users/roles (optional)",
            )
            .required(false),
        )
}

pub async fn run_add_task(
    ctx: &Context,
    command: &CommandInteraction,
    _task_orchestrator: &Arc<TaskOrchestrator>,
    timezone_service: &Arc<TimezoneService>,
) {
    let options = &command.data.options;

    // Extract parameters: notification_method, task_type, channel, mention
    let notification_method = get_string_option(options, 0).unwrap_or("DM".to_string());
    let task_type = get_string_option(options, 1).unwrap_or("single".to_string());

    // Extract channel ID if provided
    let channel_id = options
        .get(2)
        .and_then(|opt| opt.value.as_channel_id().map(|id| id.get()));

    let mention = get_string_option(options, 3).unwrap_or_default();

    // Validate channel requirement for Channel/Both notification methods - NOW STRICTER
    let requires_channel = matches!(notification_method.as_str(), "Channel" | "Both");
    if requires_channel {
        if channel_id.is_none() {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ **Channel required**\n\nFor 'Channel' or 'Both DM and Channel' notification methods, you must specify a channel")
                    .ephemeral(true),
            );
            if let Err(err) = command.create_response(&ctx.http, response).await {
                error!("Failed to send channel requirement error: {}", err);
            }
            return;
        }
    } else {
        // For DM-only, channel should not be specified
        if channel_id.is_some() {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ **Invalid channel selection**\n\nFor 'Direct Message' notification method, you can't specify a channel")
                    .ephemeral(true),
            );
            if let Err(err) = command.create_response(&ctx.http, response).await {
                error!("Failed to send channel validation error: {}", err);
            }
            return;
        } // <-- ESTA ERA LA LLAVE QUE FALTABA
    }

    // get user's timezone to display current time
    let user_id = command.user.id.get();

    // Get user's date format for dynamic placeholders
    let date_format_placeholder = match timezone_service
        .get_user_date_format_placeholder(user_id)
        .await
    {
        Ok(placeholder) => placeholder,
        Err(_) => "YYYY-MM-DD", // Default fallback
    };

    let user_timezone = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(tz)) => tz,
        _ => "UTC".to_string(),
    };

    // Get current time in user's timezone for placeholders
    let current_time_info = match timezone_service.get_current_time_for_timezone(&user_timezone) {
        Ok(time_string) => {
            // Parsear el string para extraer fecha y hora por separado
            let parts: Vec<&str> = time_string.split_whitespace().collect();
            if parts.len() == 2 {
                let date_part = parts[0].to_string();
                let time_part = parts[1].to_string();
                Some((date_part, time_part))
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // Create modal inputs
    let title_input = CreateInputText::new(InputTextStyle::Short, "Title", "task_title")
        .required(true)
        .placeholder("Enter a descriptive title for your task");

    let date_days_input = if task_type == "weekly" {
        CreateInputText::new(InputTextStyle::Short, "Days", "days")
            .required(true)
            .placeholder("Example: Mon,Wed,Fri or Monday,Wednesday,Friday")
    } else {
        // Use dynamic date placeholder based on user's format - FIXED
        let date_placeholder = if let Some((date_part, _)) = &current_time_info {
            // Use the actual current date from the timezone service (which now respects format)
            format!("Example: {}", date_part)
        } else {
            // Fallback to format examples based on user's preference
            match date_format_placeholder {
                "DD-MM-YYYY" => "Example: 27-11-2025".to_string(),
                "MM-DD-YYYY" => "Example: 11-27-2025".to_string(),
                "YYYY-MM-DD" | _ => "Example: 2025-11-27".to_string(),
            }
        };

        CreateInputText::new(InputTextStyle::Short, "Date", "date")
            .required(true)
            .placeholder(date_placeholder)
    };

    let time_placeholder = if let Some((_, time_part)) = &current_time_info {
        format!("Example: {} or your desired time", time_part)
    } else {
        "Example: 15:30".to_string()
    };

    let time_input = CreateInputText::new(InputTextStyle::Short, "Time", "time")
        .required(true)
        .placeholder(time_placeholder);

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Task Description (optional)",
        "task_description",
    )
    .required(false)
    .placeholder("Add more details about your task...");

    // Encode metadata in modal ID including channel_id
    let mention_safe = if mention.is_empty() {
        "NONE".to_string()
    } else {
        mention.replace('|', "PIPE").replace('\n', "NEWLINE")
    };

    let channel_id_str = channel_id
        .map(|id| id.to_string())
        .unwrap_or("NONE".to_string());

    let modal_custom_id = format!(
        "add_task_modal|{}|{}|{}|{}",
        task_type, notification_method, channel_id_str, mention_safe
    );

    let modal = CreateModal::new(&modal_custom_id, "📅 Create New Task").components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(date_days_input),
        CreateActionRow::InputText(time_input),
        CreateActionRow::InputText(description_input),
    ]);

    if let Err(err) = command
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        error!("Failed to show modal: {}", err);
    }
}

/// Process the modal input and create the task using TaskService
pub async fn process_task_modal_input(
    ctx: &Context,
    modal: &ModalInteraction,
    task_orchestrator: &Arc<TaskOrchestrator>,
    timezone_service: &Arc<TimezoneService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse custom_id to get task_type, notification_method, channel_id, and mention: "add_task_modal|task_type|notification_method|channel_id|mention"
    let parts: Vec<&str> = modal.data.custom_id.split('|').collect();
    if parts.len() != 5 {
        return Err("Invalid modal custom_id format - expected 5 parts".into());
    }

    let task_type = parts[1];
    let notification_method_str = parts[2];
    let channel_id_str = parts[3];
    let mention_safe = parts[4];

    // Parse channel_id - NOW VALIDATED EARLIER IN run_add_task
    let channel_id = if channel_id_str == "NONE" {
        None
    } else {
        match channel_id_str.parse::<u64>() {
            Ok(id) => Some(id),
            Err(_) => {
                let response = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("❌ Invalid channel specified")
                        .ephemeral(true),
                );
                modal.create_response(&ctx.http, response).await?;
                return Ok(());
            }
        }
    };

    // Decode mention by reversing the safe encoding
    let mention = if mention_safe == "NONE" {
        None
    } else {
        Some(mention_safe.replace("PIPE", "|").replace("NEWLINE", "\n"))
    };

    // Extract inputs from the modal (4 fields: title, date/days, time, description)
    let title = modal
        .data
        .components
        .get(0)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No title input found")?;

    let date_days_input = modal
        .data
        .components
        .get(1)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No date/days input found")?;

    let time_input = modal
        .data
        .components
        .get(2)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No time input found")?;

    let description_input = modal
        .data
        .components
        .get(3)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .unwrap_or_default(); // description is optional

    // Combine date/days and time into the expected datetime format for the orchestrator
    let datetime_input = if task_type == "weekly" {
        // PARA WEEKLY: Normalizar el formato exactamente como el parser espera
        let normalized_days = date_days_input
            .split(',')
            .map(|day| day.trim())
            .filter(|day| !day.is_empty())
            .collect::<Vec<&str>>()
            .join(",");

        let normalized_time = time_input.trim();

        // Formato exacto: "days time" con un solo espacio
        format!("{} {}", normalized_days, normalized_time)
    } else {
        // PARA SINGLE: Asegurar el formato exacto "YYYY-MM-DD HH:MM"
        let normalized_date = date_days_input.trim();
        let normalized_time = time_input.trim();

        // Formato exacto: "YYYY-MM-DD HH:MM"
        format!("{} {}", normalized_date, normalized_time)
    };

    // Parse notification method (already validated by dropdown selection)
    let notification_method = match notification_method_str {
        "DM" => NotificationMethod::DM,
        "Channel" => NotificationMethod::Channel,
        "Both" => NotificationMethod::Both,
        _ => NotificationMethod::DM,
    };

    // get user and guild info from modal
    let user_id = modal.user.id.get();
    let guild_id = modal.guild_id.map(|g| g.get()).unwrap_or(0);

    // validate that the user has timezone configured
    match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(_)) => {
            // user has timezone configured, proceed normally
        }
        Ok(None) => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ **First, setup your timezone**\n\nUse the `/timezone` command to set your location before creating tasks")
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
            return Ok(());
        }
        Err(e) => {
            error!("Error getting user timezone: {:?}", e);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Error verifying timezone")
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
            return Ok(());
        }
    }

    // Save values for response message before they're moved
    let is_channel_notification = matches!(
        notification_method,
        NotificationMethod::Channel | NotificationMethod::Both
    );
    let has_mention = mention.is_some();

    // delegate to TaskOrchestrator for business logic
    match task_orchestrator
        .handle_add_task_modal(
            user_id,
            guild_id,
            task_type,
            title.clone(),
            description_input,
            notification_method,
            datetime_input,
            channel_id,
            mention,
        )
        .await
    {
        Ok(_task_id) => {
            let response_content = if is_channel_notification && has_mention {
                format!(
                    "✅ Task **{}** created successfully with mention in specified channel!",
                    title
                )
            } else if is_channel_notification {
                format!(
                    "✅ Task **{}** created successfully! Notifications will be sent to the specified channel",
                    title
                )
            } else {
                format!(
                    "✅ Task **{}** created successfully! You will receive DM notifications",
                    title
                )
            };

            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default().content(response_content),
            );
            modal.create_response(&ctx.http, response).await?;
        }
        Err(error) => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content(format!("❌ {}", error))
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
        }
    }

    Ok(())
}
