use crate::application::domain::Recurrence;
use crate::application::repositories::task_repository::TaskRepository;
use chrono::{TimeZone, Datelike, NaiveDateTime, Timelike, Utc};
use serenity::{
    all::{
        ActionRowComponent, CommandDataOptionValue, CommandInteraction, CommandOptionType,
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage, InputTextStyle, ModalInteraction,
    },
    builder::{CreateActionRow, CreateInputText, CreateModal},
    prelude::*,
};
use std::sync::Arc;

// Registers /add_task command
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
}

// Execute /add_task command logic
pub async fn run_add_task(ctx: &Context, command: &CommandInteraction, _repo: &TaskRepository) {
    let options = &command.data.options;

    // extract message
    let message = match options.get(0) {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::String(s) => s.clone(),
            _ => {
                let builder = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("❌ Invalid message")
                        .ephemeral(true),
                );
                let _ = command.create_response(&ctx.http, builder).await;
                return;
            }
        },
        None => {
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing message")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, builder).await;
            return;
        }
    };

    // extract task_type
    let task_type = match options.get(1) {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::String(s) => s.as_str(),
            _ => {
                let builder = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("❌ Invalid task type")
                        .ephemeral(true),
                );
                let _ = command.create_response(&ctx.http, builder).await;
                return;
            }
        },
        None => {
            let builder = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("❌ Missing task type")
                    .ephemeral(true),
            );
            let _ = command.create_response(&ctx.http, builder).await;
            return;
        }
    };

    // weekly task → launch modal
    if task_type == "weekly" {
        let input_text = CreateInputText::new(
            InputTextStyle::Short,
            "Enter days and hour (Mon,Wed,Fri 14:00)",
            "Format: Mon,Wed,Fri 14:30",
        )
        .required(true);

        let action_row = CreateActionRow::InputText(input_text);

        let modal = CreateModal::new(
            &format!("weekly_task_modal|{}", message),
            "📅 Set Weekly Task",
        )
        .components(vec![action_row]);

        if let Err(err) = command
            .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
            .await
        {
            eprintln!("Failed to show weekly modal: {}", err);
        }
        return;
    }

    // single task → launch modal
    if task_type == "single" {
        let input_text = CreateInputText::new(
            InputTextStyle::Short,
            "Enter date (Year-Month-Day Hour:Minutes)",
            "Enter date & time (YYYY-MM-DD HH:MM)",
        )
        .required(true);

        let action_row = CreateActionRow::InputText(input_text);

        let modal = CreateModal::new(&format!("single_task_modal|{}", message), "📅 Set Task")
            .components(vec![action_row]);

        if let Err(err) = command
            .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
            .await
        {
            eprintln!("Failed to show single modal: {}", err);
        }
    }
}

// Process the response from the "single" task modal
pub async fn process_single_task_input(
    ctx: &Context,
    modal: &ModalInteraction,
    repo: &Arc<TaskRepository>,
    message: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let date_time_str: String = match modal.data.components.get(0) {
        Some(row) => match row.components.get(0) {
            Some(ActionRowComponent::InputText(input)) => match &input.value {
                Some(val) => val.clone(),
                None => return Err("No input value found".into()),
            },
            _ => return Err("No input value found".into()),
        },
        None => return Err("No input value found".into()),
    };

    let naive_dt = NaiveDateTime::parse_from_str(&date_time_str, "%Y-%m-%d %H:%M")
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

    let task_id = repo.add_task(modal.user.id.get(), message, Some(dt_utc), None);

    if let Err(err) = repo.save_all() {
        eprintln!("Failed to save tasks to JSON: {}", err);
    }

    let response_content = format!("✅ Single task **#{}** created for {}", task_id, dt_utc);
    let builder = CreateInteractionResponseMessage::default().content(response_content);

    modal
        .create_response(ctx, CreateInteractionResponse::Message(builder))
        .await?;

    Ok(())
}

// Process the response from the "weekly" task modal
pub async fn process_weekly_task_input(
    ctx: &Context,
    modal: &ModalInteraction,
    repo: &Arc<TaskRepository>,
    message: String,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::application::commands::utils::weekly_parser::parse_weekly_input;

    // extract input from modal
    let input_str: String = match modal.data.components.get(0) {
        Some(row) => match row.components.get(0) {
            Some(ActionRowComponent::InputText(input)) => match &input.value {
                Some(val) => val.clone(),
                None => return Err("No input value found".into()),
            },
            _ => return Err("No input value found".into()),
        },
        None => return Err("No input value found".into()),
    };

    // flexible parser
    let (days, hour, minute, formatted_str) = parse_weekly_input(&input_str)?;
    println!("Parsed weekly input: {}", formatted_str);

    // calculate first occurrence from now
    let now = Utc::now();
    let mut first_time = now;
    while !days.contains(&first_time.weekday()) {
        first_time = first_time + chrono::Duration::days(1);
    }

    first_time = first_time
        .with_hour(hour as u32)
        .and_then(|t| t.with_minute(minute as u32))
        .unwrap_or(first_time);

    // check if first occurrence is in the past
    if first_time < Utc::now() {
        let builder = CreateInteractionResponseMessage::default()
            .content("❌ Cannot create a weekly task in the past.")
            .ephemeral(true);
        modal
            .create_response(ctx, CreateInteractionResponse::Message(builder))
            .await?;
        return Ok(());
    }

    // create recurrence
    let recurrence = Some(Recurrence::Weekly { days, hour, minute });

    // add task to repo
    let task_id = repo.add_task(
        modal.user.id.get(),
        message,
        Some(first_time),
        recurrence.clone(),
    );

    if let Err(err) = repo.save_all() {
        eprintln!("Failed to save tasks to JSON: {}", err);
    }

    println!("Weekly task created with ID {}", task_id);

    let response_content = format!(
        "✅ Weekly task **#{}** created for {}",
        task_id, formatted_str
    );

    let builder = CreateInteractionResponseMessage::default().content(response_content);

    modal
        .create_response(ctx, CreateInteractionResponse::Message(builder))
        .await?;

    Ok(())
}
