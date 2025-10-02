use crate::application::repositories::task_repository::TaskRepository;
use chrono::{DateTime, NaiveDateTime, Utc};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateCommand,
        CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
        InputTextStyle, ModalInteraction, ActionRowComponent,
    },
    builder::{CreateActionRow, CreateInputText, CreateModal},
    prelude::*,
};
use std::sync::Arc;

/// Registers /add_task command
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
                "Task type: 'single' or 'daily'",
            )
            .add_string_choice("Single (specific date/time)", "single")
            .add_string_choice("Daily (repeats every day)", "daily")
            .required(true),
        )
}

/// Execute /add_task command logic
pub async fn run_add_task(ctx: &Context, command: &CommandInteraction, repo: &TaskRepository) {
    let options = &command.data.options;

    // --- 1️⃣ Extraer el mensaje ---
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

    // --- 2️⃣ Extraer el tipo de tarea ---
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

    // --- 3️⃣ Tarea diaria ---
    if task_type == "daily" {
        let now = Utc::now();
        let task_id = repo.add_task(command.user.id.get(), message.clone(), now, true);

        // Guardar en JSON
        if let Err(err) = repo.save_all() {
            eprintln!("❌ Failed to save tasks to JSON: {}", err);
        }

        let response_content = format!("✅ Daily task **#{}** created: {}", task_id, message);
        let builder = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default().content(response_content),
        );
        let _ = command.create_response(&ctx.http, builder).await;
        return;
    }

    // --- 4️⃣ Tarea única → lanzar modal ---
    if task_type == "single" {
        let input_text = CreateInputText::new(
            InputTextStyle::Short,
            "Enter date & time (Year-Month-Day Hour:Minutes)",
            "Enter date & time (YYYY-MM-DD HH:MM)",
        )
        .required(true);

        let action_row = CreateActionRow::InputText(input_text);

        let modal = CreateModal::new(
            &format!("single_task_modal|{}", message), // guardamos la descripción en el custom_id
            "📅 Set Task",
        )
        .components(vec![action_row]);

        if let Err(err) = command
            .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
            .await
        {
            eprintln!("❌ Failed to show modal: {}", err);
        }
    }
}

/// Procesa la respuesta del usuario del modal para la tarea "single"
pub async fn process_single_task_input(
    ctx: &Context,
    modal: &ModalInteraction,
    repo: &Arc<TaskRepository>,
    message: String, // ahora se recibe la descripción de la tarea directamente
) -> Result<(), Box<dyn std::error::Error>> {
    // 1️⃣ Extraer el valor del input
    let date_time_str: String = match modal.data.components.get(0) {
        Some(row) => match row.components.get(0) {
            Some(ActionRowComponent::InputText(input)) => match &input.value {
                Some(val) => val.clone(),
                None => return Err(Box::<dyn std::error::Error>::from("No input value found")),
            },
            _ => return Err(Box::<dyn std::error::Error>::from("No input value found")),
        },
        None => return Err(Box::<dyn std::error::Error>::from("No input value found")),
    };

    // 2️⃣ Parsear a NaiveDateTime
    let naive_dt = NaiveDateTime::parse_from_str(&date_time_str, "%Y-%m-%d %H:%M")
        .map_err(|_| "Failed to parse date/time. Use YYYY-MM-DD HH:MM")?;

    // 3️⃣ Convertir a DateTime<Utc>
    let dt_utc: DateTime<Utc> = DateTime::<Utc>::from_utc(naive_dt, Utc);

    // 4️⃣ Crear la tarea en el repositorio
    let task_id = repo.add_task(modal.user.id.get(), message, dt_utc, false);

    // Guardar en JSON
    if let Err(err) = repo.save_all() {
        eprintln!("❌ Failed to save tasks to JSON: {}", err);
    }

    // 5️⃣ Enviar confirmación al usuario
    let response_content = format!("✅ Single task **#{}** created for {}", task_id, dt_utc);
    let builder = CreateInteractionResponseMessage::default().content(response_content);

    modal
        .create_response(ctx, CreateInteractionResponse::Message(builder))
        .await?;

    Ok(())
}
