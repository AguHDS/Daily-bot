use crate::application::commands::utils::{
    get_string_option, notification_method_as_str, parse_notification_method,
};
use crate::application::services::task_service::TaskService;
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

/// Register /add_task command with notification option
pub fn register_add_task_command() -> CreateCommand {
    CreateCommand::new("add_task")
        .description("Add a new task")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "message", "Task description")
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
}

/// Run /add_task and launch modal for date/time input
pub async fn run_add_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>, // 🆕 Nuevo parámetro
) {
    let options = &command.data.options;

    // extract message
    let message = match get_string_option(options, 0) {
        Some(msg) => msg,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing or invalid message")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, response).await;
            return;
        }
    };

    // extract task type and notification method
    let task_type = get_string_option(options, 1).unwrap_or("single".to_string());
    let notification_method = get_string_option(options, 2)
        .map(|s| parse_notification_method(&s))
        .unwrap_or(NotificationMethod::DM);

    // 🆕 Obtener la timezone del usuario para mostrar en el placeholder
    let user_id = command.user.id.get();
    let user_timezone_info = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(timezone)) => match timezone_service.get_current_time_for_timezone(&timezone) {
            Ok(current_time) => format!("{}", current_time),
            Err(_) => "".to_string(),
        },
        _ => "".to_string(),
    };

    // create input text depending on task type
    let input_text = if task_type == "weekly" {
        CreateInputText::new(
            InputTextStyle::Short,
            "Enter Days & Hour (Mon,Wed,Fri HH:MM)",
            "weekly_datetime", // custom_id
        )
        .required(true)
        .placeholder(format!("{}", ""))
    } else {
        CreateInputText::new(
            InputTextStyle::Short,
            "Enter Date & Time (YYYY-MM-DD HH:MM)",
            "single_datetime", // custom_id
        )
        .required(true)
        .placeholder(format!("{}", user_timezone_info))
    };

    let action_row = CreateActionRow::InputText(input_text);

    // Pass task data in custom_id for modal processing
    let modal_custom_id = format!(
        "{}_task_modal|{}|{}",
        task_type,
        message.replace('|', "-"), // Sanitize to avoid parsing issues
        notification_method_as_str(&notification_method)
    );

    let modal = CreateModal::new(&modal_custom_id, "📅 Set Task").components(vec![action_row]);

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
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>, // 🆕 Nuevo parámetro
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse custom_id: "single_task_modal|message|NotificationMethod"
    let parts: Vec<&str> = modal.data.custom_id.split('|').collect();
    if parts.len() != 3 {
        return Err("Invalid modal custom_id format".into());
    }

    let task_type = parts[0].strip_suffix("_task_modal").unwrap_or("single");
    let message = parts[1].to_string();
    let notification_method = parse_notification_method(parts[2]);

    // extract user input from modal
    let input_str = modal
        .data
        .components
        .get(0)
        .and_then(|row| row.components.get(0))
        .and_then(|c| match c {
            ActionRowComponent::InputText(input) => input.value.clone(),
            _ => None,
        })
        .ok_or("No input value found")?;

    // get user and guild info from modal
    let user_id = modal.user.id.get();
    let guild_id = modal.guild_id.map(|g| g.get()).unwrap_or(0);

    // 🆕 Validar que el usuario tenga timezone configurada
    match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(_)) => {
            // Usuario tiene timezone configurada, proceder normalmente
        }
        Ok(None) => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ **Primero configura tu zona horaria**\n\nUsa el comando `/timezone` para configurar tu ubicación antes de crear tareas.")
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
            return Ok(());
        }
        Err(e) => {
            eprintln!("Error getting user timezone: {:?}", e);
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Error al verificar tu zona horaria")
                    .ephemeral(true),
            );
            modal.create_response(&ctx.http, response).await?;
            return Ok(());
        }
    }

    // delegate to TaskService for business logic - 🆕 Pasar timezone_service
    match task_service
        .handle_add_task_modal(
            user_id,
            guild_id,
            task_type,
            message,
            notification_method,
            input_str,
            timezone_service.clone(), // 🆕 Pasar timezone_service
        )
        .await
    {
        Ok(task_id) => {
            let response_content = format!("✅ Task **#{}** created successfully!", task_id);
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
