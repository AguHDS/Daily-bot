
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
pub fn register_add_task_command() -> CreateCommand {
    CreateCommand::new("add_task")
        .description("Add a new task")
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
                CommandOptionType::String,
                "notification_method",
                "How to notify you when the task is due",
            )
            .add_string_choice("Direct Message", "DM")
            .add_string_choice("Channel Notification", "Channel")
            .add_string_choice("Both DM and Channel", "Both")
            .required(true),
        )
}

pub async fn run_add_task(
    ctx: &Context,
    command: &CommandInteraction,
    _task_orchestrator: &Arc<TaskOrchestrator>,
    timezone_service: &Arc<TimezoneService>,
) {
    let options = &command.data.options;
    
    // Extract task_type and notification_method from command options
    let task_type = get_string_option(options, 0).unwrap_or("single".to_string());
    let notification_method = get_string_option(options, 1).unwrap_or("DM".to_string());
    
    // get user's timezone to display in the datetime placeholder
    let user_id = command.user.id.get();
    let user_timezone_info = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(timezone)) => match timezone_service.get_current_time_for_timezone(&timezone) {
            Ok(current_time) => format!("{}", current_time),
            Err(_) => "".to_string(),
        },
        _ => "".to_string(),
    };

    // Create modal inputs for remaining fields (title, datetime, description)
    let title_input = CreateInputText::new(
        InputTextStyle::Short,
        "Title",
        "task_title",
    )
    .required(true)
    .placeholder("Enter a descriptive title for your task");

    let datetime_input = CreateInputText::new(
        InputTextStyle::Short,
        if task_type == "weekly" { "Days & Time (e.g., Mon,Wed,Fri 14:30)" } else { "Date & Time (YYYY-MM-DD HH:MM)" },
        "datetime",
    )
    .required(true)
    .placeholder(if task_type == "weekly" {
        "Example: Mon,Wed,Fri 14:30".to_string()
    } else if user_timezone_info.is_empty() {
        "Example: 2025-11-01 15:30".to_string()
    } else {
        format!("Current time in your timezone: {}", user_timezone_info)
    });

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "Task Description (optional)",
        "task_description",
    )
    .required(false)
    .placeholder("Add more details about your task...");

    // Encode task_type and notification_method in modal custom_id for processing
    let modal_custom_id = format!("add_task_modal|{}|{}", task_type, notification_method);

    // Create modal with remaining input fields
    let modal = CreateModal::new(&modal_custom_id, "📅 Create New Task")
        .components(vec![
            CreateActionRow::InputText(title_input),
            CreateActionRow::InputText(datetime_input),
            CreateActionRow::InputText(description_input),
        ]);

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
    // Parse custom_id to get task_type and notification_method: "add_task_modal|task_type|notification_method"
    let parts: Vec<&str> = modal.data.custom_id.split('|').collect();
    if parts.len() != 3 {
        return Err("Invalid modal custom_id format".into());
    }
    
    let task_type = parts[1];
    let notification_method_str = parts[2];
    
    // Extract inputs from the modal (3 fields: title, datetime, description)
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

    let datetime_input = modal
        .data
        .components
        .get(1)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No datetime input found")?;

    let description_input = modal
        .data
        .components
        .get(2)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .unwrap_or_default(); // description is optional

    // Parse notification method (already validated by dropdown selection)
    let notification_method = match notification_method_str {
        "DM" => NotificationMethod::DM,
        "Channel" => NotificationMethod::Channel,
        "Both" => NotificationMethod::Both,
        _ => NotificationMethod::DM, // fallback, though this shouldn't happen with dropdowns
    };

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
            &task_type,
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
