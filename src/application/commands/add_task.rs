use crate::application::commands::utils::{
    get_string_option, notification_method_as_str, parse_notification_method, parse_weekly_input,
};
use crate::application::domain::{NotificationMethod, Recurrence, Task};
use crate::application::repositories::task_repository::TaskRepository;
use chrono::{Datelike, NaiveDateTime, TimeZone, Timelike, Utc};
use serenity::{
    all::{
        ActionRowComponent, CommandInteraction, CommandOptionType,
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage, InputTextStyle, ModalInteraction,
    },
    builder::{CreateActionRow, CreateInputText, CreateModal},
    prelude::*,
};
use std::sync::Arc;

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

pub async fn run_add_task(
    ctx: &Context,
    command: &CommandInteraction,
    _repo: &Arc<dyn TaskRepository>,
) {
    let options = &command.data.options;

    use serenity::all::CreateInteractionResponse;
    use serenity::builder::CreateInteractionResponseMessage;

    // extract message
    let message = match get_string_option(options, 0) {
        Some(msg) => msg,
        None => {
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing or invalid message")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, builder).await;
            return;
        }
    };

    // extract task type
    let task_type = match get_string_option(options, 1) {
        Some(s) => s,
        None => {
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing or invalid task type")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, builder).await;
            return;
        }
    };

    // extract notification_method
    let notification_method = match get_string_option(&options, 2) {
        Some(s) => match s.as_str() {
            "DM" => NotificationMethod::DM,
            "Channel" => NotificationMethod::Channel,
            "Both" => NotificationMethod::Both,
            _ => NotificationMethod::DM, // default value if unknown
        },
        None => NotificationMethod::DM, // default value if missing
    };

    // launch modal
    let input_text = if task_type == "weekly" {
        CreateInputText::new(
            InputTextStyle::Short,
            "Enter days and hour (Mon,Wed,Fri 14:00)",
            "Format: Mon,Wed,Fri 14:30",
        )
        .required(true)
    } else {
        CreateInputText::new(
            InputTextStyle::Short,
            "Enter date (YYYY-MM-DD HH:MM)",
            "Enter date & time (YYYY-MM-DD HH:MM)",
        )
        .required(true)
    };

    let action_row = CreateActionRow::InputText(input_text);

    let modal_custom_id = format!(
        "{}_task_modal|{}|{}",
        task_type,
        message.replace('|', "-"), // send problems with separator
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

/// Process the response from a modal
pub async fn process_task_modal_input(
    ctx: &Context,
    modal: &ModalInteraction,
    repo: &Arc<dyn TaskRepository>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // parse custom_id: "single_task_modal|message|NotificationMethod"
    let parts: Vec<&str> = modal.data.custom_id.split('|').collect();
    if parts.len() != 3 {
        return Err("Invalid modal custom_id format".into());
    }

    let task_type = parts[0].strip_suffix("_task_modal").unwrap_or("single");
    let message = parts[1].to_string();
    let notification_method = parse_notification_method(parts[2]);

    // extract input text
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

    if task_type == "single" {
        let naive_dt = NaiveDateTime::parse_from_str(&input_str, "%Y-%m-%d %H:%M")
            .map_err(|_| "Failed to parse date/time. Use YYYY-MM-DD HH:MM")?;
        let dt_utc = Utc.from_utc_datetime(&naive_dt);

        if dt_utc < Utc::now() {
            let builder = CreateInteractionResponseMessage::default()
                .content("❌ Cannot create a task in the past.")
                .ephemeral(true);
            modal
                .create_response(ctx, CreateInteractionResponse::Message(builder))
                .await?;
            return Ok(());
        }

        let task = Task::new(
            0,
            modal.user.id.get(),
            message,
            Some(dt_utc),
            None,
            notification_method,
        );
        let task_id = repo.add_task(task)?;

        let response_content = format!("✅ Single task **#{}** created for {}", task_id, dt_utc);
        let builder = CreateInteractionResponseMessage::default().content(response_content);
        modal
            .create_response(ctx, CreateInteractionResponse::Message(builder))
            .await?;
    } else {
        // weekly task
        let (days, hour, minute, formatted_str) = parse_weekly_input(&input_str).map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(format!("{}", e)) },
        )?;

        let now = Utc::now();
        let mut first_time = now;
        while !days.contains(&first_time.weekday()) {
            first_time = first_time + chrono::Duration::days(1);
        }

        first_time = first_time
            .with_hour(hour as u32)
            .and_then(|t| t.with_minute(minute as u32))
            .ok_or("Invalid hour/minute")?;

        if first_time < Utc::now() {
            let builder = CreateInteractionResponseMessage::default()
                .content("❌ Cannot create a weekly task in the past.")
                .ephemeral(true);
            modal
                .create_response(ctx, CreateInteractionResponse::Message(builder))
                .await?;
            return Ok(());
        }

        let recurrence = Some(Recurrence::Weekly { days, hour, minute });
        let task = Task::new(
            0,
            modal.user.id.get(),
            message,
            Some(first_time),
            recurrence,
            notification_method,
        );
        let task_id = repo.add_task(task)?;

        let response_content = format!(
            "✅ Weekly task **#{}** created for {}",
            task_id, formatted_str
        );
        let builder = CreateInteractionResponseMessage::default().content(response_content);
        modal
            .create_response(ctx, CreateInteractionResponse::Message(builder))
            .await?;
    }

    Ok(())
}
