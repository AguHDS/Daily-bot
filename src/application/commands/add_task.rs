use crate::application::domain::Recurrence;
use crate::application::repositories::task_repository::TaskRepository;
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc, Weekday};
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
pub async fn run_add_task(ctx: &Context, command: &CommandInteraction, repo: &TaskRepository) {
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

    let dt_utc: DateTime<Utc> = DateTime::<Utc>::from_utc(naive_dt, Utc);

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

    let parts: Vec<&str> = input_str.split_whitespace().collect();
    if parts.len() != 2 {
        eprintln!("Invalid input format: {:?}", parts);
        return Err("Invalid format. Use: Mon,Wed,Fri HH:MM".into());
    }

    let days_str = parts[0];
    let time_str = parts[1];

    let mut days: Vec<Weekday> = Vec::new();
    for day in days_str.split(',') {
        match day.to_lowercase().as_str() {
            "mon" => days.push(Weekday::Mon),
            "tue" => days.push(Weekday::Tue),
            "wed" => days.push(Weekday::Wed),
            "thu" => days.push(Weekday::Thu),
            "fri" => days.push(Weekday::Fri),
            "sat" => days.push(Weekday::Sat),
            "sun" => days.push(Weekday::Sun),
            _ => {
                eprintln!("Invalid weekday in input: {}", day);
                return Err(format!("Invalid weekday: {}", day).into());
            }
        }
    }

    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 2 {
        eprintln!("Invalid time format: {:?}", time_parts);
        return Err("Invalid time format. Use HH:MM".into());
    }

    let hour: u8 = time_parts[0].parse()?;
    let minute: u8 = time_parts[1].parse()?;

    let now = Utc::now();
    let mut first_time = now;

    while !days.contains(&first_time.weekday()) {
        first_time = first_time + chrono::Duration::days(1);
    }

    first_time = first_time
        .with_hour(hour as u32)
        .and_then(|t| t.with_minute(minute as u32))
        .unwrap_or(first_time);

    let recurrence = Some(Recurrence::Weekly { days, hour, minute });

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
        "✅ Weekly task **#{}** created on {:?} at {:02}:{:02}",
        task_id,
        recurrence.as_ref().unwrap(),
        hour,
        minute
    );

    let builder = CreateInteractionResponseMessage::default().content(response_content);

    modal
        .create_response(ctx, CreateInteractionResponse::Message(builder))
        .await?;

    Ok(())
}
