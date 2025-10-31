use crate::application::commands::utils::{
    get_string_option, notification_method_as_str, parse_notification_method,
};
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
pub fn register_add_task_command() -> CreateCommand {
    CreateCommand::new("add_task")
        .description("Add a new task")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "title", "Task title")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "task_type",
                "Task type: 'single' or 'weekly'",
            )
            .add_string_choice("Single (specific date/time)", "single")
            .add_string_choice("Weekly (repeats on specific days and hour)", "weekly")
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "notify",
                "Notification method: DM, Channel, Both",
            )
            .add_string_choice("DM", "DM")
            .add_string_choice("Channel", "Channel")
            .add_string_choice("Both", "Both")
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "description",
                "Task description (optional)",
            )
            .required(false),
        )
}

pub async fn run_add_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_orchestrator: &Arc<TaskOrchestrator>,
    timezone_service: &Arc<TimezoneService>,
) {
    let options = &command.data.options;

    // extract title (required) - usar índice 0 (puede ser "message" o "title")
    let title = match get_string_option(options, 0) {
        Some(title) => title,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing or invalid title")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, response).await;
            return;
        }
    };

    let task_type = get_string_option(options, 1).unwrap_or("single".to_string());
    let notification_method = get_string_option(options, 2)
        .map(|s| parse_notification_method(&s))
        .unwrap_or(NotificationMethod::DM);

    if task_type != "single" && task_type != "weekly" {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default()
                .content("❌ Invalid task type. Must be 'single' or 'weekly'")
                .ephemeral(true),
        );
        let _ = command.create_response(&ctx.http, response).await;
        return;
    }

    // get user's timezone to display in the placeholder
    let user_id = command.user.id.get();
    let user_timezone_info = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(timezone)) => match timezone_service.get_current_time_for_timezone(&timezone) {
            Ok(current_time) => format!("{}", current_time),
            Err(_) => "".to_string(),
        },
        _ => "".to_string(),
    };

    let datetime_input = if task_type == "weekly" {
        CreateInputText::new(
            InputTextStyle::Short,
            "Format: Mon,Tue,Wed,Thu,Fri,Sat,Sun 16:00",
            "weekly_datetime",
        )
        .required(true)
        .placeholder("Example: Mon,Wed,Fri 14:30")
    } else {
        CreateInputText::new(
            InputTextStyle::Short,
            "Enter Date & Time (YYYY-MM-DD HH:MM)",
            "single_datetime",
        )
        .required(true)
        .placeholder(format!("{}", user_timezone_info))
    };

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Task Description (optional)",
        "task_description",
    )
    .required(false)
    .placeholder("Add more details about your task...");

    let datetime_row = CreateActionRow::InputText(datetime_input);
    let description_row = CreateActionRow::InputText(description_input);

    // pass task data in custom_id for modal processing
    let modal_custom_id = format!(
        "{}_task_modal|{}|{}",
        task_type,
        title.replace('|', "-"), // sanitize to avoid parsing issues
        notification_method_as_str(&notification_method)
    );

    let modal = CreateModal::new(&modal_custom_id, "📅 Create Task")
        .components(vec![datetime_row, description_row]);

    if let Err(err) = command
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await
    {
        eprintln!("Failed to show modal: {}", err);
    }
}

/// Process the modal input and create the task using TaskService
pub async fn process_task_modal_input(
    ctx: &Context,
    modal: &ModalInteraction,
    task_orchestrator: &Arc<TaskOrchestrator>,
    timezone_service: &Arc<TimezoneService>,
    config_service: &Arc<crate::application::services::config_service::ConfigService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // parse custom_id: "single_task_modal|title|NotificationMethod"
    let parts: Vec<&str> = modal.data.custom_id.split('|').collect();
    if parts.len() != 3 {
        return Err("Invalid modal custom_id format".into());
    }

    let task_type = parts[0].strip_suffix("_task_modal").unwrap_or("single");
    let title = parts[1].to_string();
    let notification_method = parse_notification_method(parts[2]);

    // extract both inputs from modal
    let datetime_input = modal
        .data
        .components
        .get(0)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No datetime input found")?;

    let description_input = modal
        .data
        .components
        .get(1)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .unwrap_or_default(); // description optional

    // get user and guild info from modal
    let user_id = modal.user.id.get();
    let guild_id = modal.guild_id.map(|g| g.get()).unwrap_or(0);

    // validate notification channel is configured if needed
    match notification_method {
        NotificationMethod::Channel | NotificationMethod::Both => {
            match config_service.get_notification_channel(guild_id).await {
                None => {
                    let response = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("❌ **No notification channel configured**\n\nTo create tasks with channel notifications, an admin must first set up a notification channel using `/set_notification_channel`")
                            .ephemeral(true),
                    );
                    modal.create_response(&ctx.http, response).await?;
                    return Ok(());
                }
                Some(_) => {
                    // Channel is configured, continue with task creation
                }
            }
        }
        NotificationMethod::DM => {
            // DM notifications don't require channel configuration
        }
    }

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
            eprintln!("Error getting user timezone: {:?}", e);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Error verifying timezone")
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
            return Ok(());
        }
    }

    // delegate to TaskOrchestrator for business logic - now passing title and description
    match task_orchestrator
        .handle_add_task_modal(
            user_id,
            guild_id,
            task_type,
            title.clone(),
            description_input,
            notification_method,
            datetime_input,
        )
        .await
    {
        Ok(_task_id) => {
            let response_content = format!("✅ Task **{}** created successfully!", title);
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
