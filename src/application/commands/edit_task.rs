use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use crate::domain::entities::task::Recurrence;
use crate::domain::value_objects::weekday_format::WeekdayFormat;
use chrono::{Timelike, Utc};
use serenity::all::{
    ActionRowComponent, CommandInteraction, ComponentInteraction, ComponentInteractionDataKind,
    Context, CreateActionRow, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, InputTextStyle, ModalInteraction, CreateEmbed
};
use serenity::builder::{
    CreateInputText, CreateModal, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use std::sync::Arc;

/// Register /edit_task command
pub fn register_edit_task_command() -> CreateCommand {
    CreateCommand::new("edit_task").description("Edit your task")
}

/// Extract time part from local time string (HH:MM from "YYYY-MM-DD HH:MM")
fn extract_time_part(local_time: &str, hour: u8, minute: u8) -> String {
    local_time
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{:02}:{:02}", hour, minute))
}

/// Format UTC time to local time string and extract time part
fn format_utc_time_to_local_time(
    timezone_service: &TimezoneService,
    utc_time: chrono::DateTime<Utc>,
    user_timezone: &str,
    hour: u8,
    minute: u8,
) -> String {
    match timezone_service.format_from_utc_with_timezone(utc_time, user_timezone) {
        Ok(local_time) => extract_time_part(&local_time, hour, minute),
        Err(_) => format!("{:02}:{:02}", hour, minute),
    }
}

/// Create UTC time from hour and minute
fn create_utc_time(hour: u8, minute: u8) -> chrono::DateTime<Utc> {
    Utc::now()
        .with_hour(hour as u32)
        .and_then(|t| t.with_minute(minute as u32))
        .and_then(|t| t.with_second(0))
        .unwrap()
}

/// Format days and time for display
fn format_days_and_time(
    days: &[chrono::Weekday],
    hour: u8,
    minute: u8,
    timezone_service: &TimezoneService,
    user_timezone: &str,
) -> String {
    let days_str = days
        .iter()
        .map(|d| d.to_short_en())
        .collect::<Vec<_>>()
        .join(",");

    let utc_time = create_utc_time(hour, minute);
    let time_part =
        format_utc_time_to_local_time(timezone_service, utc_time, user_timezone, hour, minute);

    format!("{} {}", days_str, time_part)
}

/// Format task date for final display
fn format_task_date(
    task: &crate::domain::entities::task::Task,
    timezone_service: &TimezoneService,
    user_timezone: &str,
) -> String {
    if let Some(Recurrence::Weekly { days, hour, minute }) = &task.recurrence {
        let days_str = days
            .iter()
            .map(|d| d.to_short_en())
            .collect::<Vec<_>>()
            .join(",");

        let utc_time = create_utc_time(*hour, *minute);
        let time_part = format_utc_time_to_local_time(
            timezone_service,
            utc_time,
            user_timezone,
            *hour,
            *minute,
        );

        format!("{} at {}", days_str, time_part)
    } else if let Some(dt) = task.scheduled_time {
        match timezone_service.format_from_utc_with_timezone(dt, user_timezone) {
            Ok(local_time) => local_time,
            Err(_) => dt.format("%Y-%m-%d %H:%M").to_string(),
        }
    } else {
        "Date missing".to_string()
    }
}

/// Run /edit_task, show select menus for single and weekly tasks
pub async fn run_edit_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) {
    let user_id = command.user.id.get();

    // verify user's timezone first
    let user_timezone = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(tz)) => tz,
        Ok(None) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("❌ **First, setup your timezone**\n\nUse `/timezone` to set your location before editing tasks")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
        Err(_) => "UTC".to_string(),
    };

    // delegate to TaskService for business logic
    let (single_tasks, weekly_tasks) = task_service.get_user_tasks_for_editing(user_id).await;

    if single_tasks.is_empty() && weekly_tasks.is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("You don't have any task to edit"),
                ),
            )
            .await;
        return;
    }

    let mut components: Vec<CreateActionRow> = Vec::new();

    if !single_tasks.is_empty() {
        let options = single_tasks
            .iter()
            .map(|task| {
                let display_title = if task.title.len() > 30 {
                    format!("{}...", &task.title[..30])
                } else {
                    task.title.clone()
                };

                let label = if let Some(dt) = task.scheduled_time {
                    match timezone_service.format_from_utc_with_timezone(dt, &user_timezone) {
                        Ok(local_time) => {
                            format!("#{}: {} (Single on {})", task.id, display_title, local_time)
                        }
                        Err(_) => format!(
                            "#{}: {} (Single on {})",
                            task.id,
                            display_title,
                            dt.format("%Y-%m-%d %H:%M")
                        ),
                    }
                } else {
                    format!("#{}: {}", task.id, display_title)
                };
                CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect::<Vec<_>>();

        let select =
            CreateSelectMenu::new("edit_menu_single", CreateSelectMenuKind::String { options })
                .placeholder("Single tasks")
                .min_values(1)
                .max_values(1);

        components.push(CreateActionRow::SelectMenu(select));
    }

    if !weekly_tasks.is_empty() {
        let options = weekly_tasks
            .iter()
            .map(|task| {
                let display_title = if task.title.len() > 30 {
                    format!("{}...", &task.title[..30])
                } else {
                    task.title.clone()
                };

                let label =
                    if let Some(Recurrence::Weekly { days, hour, minute }) = &task.recurrence {
                        let days_str = days
                            .iter()
                            .map(|d| d.to_short_en())
                            .collect::<Vec<_>>()
                            .join(",");

                        let time_part = format_utc_time_to_local_time(
                            timezone_service,
                            create_utc_time(*hour, *minute),
                            &user_timezone,
                            *hour,
                            *minute,
                        );

                        format!(
                            "#{}: {} (Weekly on {} at {})",
                            task.id, display_title, days_str, time_part
                        )
                    } else {
                        format!("#{}: {}", task.id, display_title)
                    };

                CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect::<Vec<_>>();

        let select =
            CreateSelectMenu::new("edit_menu_weekly", CreateSelectMenuKind::String { options })
                .placeholder("Weekly tasks")
                .min_values(1)
                .max_values(1);

        components.push(CreateActionRow::SelectMenu(select));
    }

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("Select a task to edit:")
                    .components(components),
            ),
        )
        .await;
}

/// Show modal to edit selected task
pub async fn handle_edit_select(
    ctx: &Context,
    interaction: &ComponentInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) {
    let selected_id = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            if let Some(val) = values.first() {
                match val.parse::<u64>() {
                    Ok(id) => id,
                    Err(_) => {
                        let _ = interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::default()
                                        .content("❌ Invalid selection."),
                                ),
                            )
                            .await;
                        return;
                    }
                }
            } else {
                return;
            }
        }
        _ => return,
    };

    let user_id = interaction.user.id.get();

    // get user's timezone for placeholder
    let user_timezone = match timezone_service.get_user_timezone(user_id).await {
        Ok(Some(tz)) => tz,
        Ok(None) => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("❌ Please set your timezone first with `/timezone`"),
                    ),
                )
                .await;
            return;
        }
        Err(_) => "UTC".to_string(),
    };

    // delegate to TaskService for business logic
    let task = match task_service
        .get_task_for_editing(selected_id, user_id)
        .await
    {
        Some(t) => t,
        None => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default().content(
                            "❌ Couldn't find the task or you don't have permission to edit it.",
                        ),
                    ),
                )
                .await;
            return;
        }
    };

    let title_input = CreateInputText::new(InputTextStyle::Short, "New title", "new_title")
        .placeholder(&task.title)
        .required(false);

    let description_input = CreateInputText::new(
        InputTextStyle::Paragraph,
        "New description (optional)",
        "new_description",
    )
    .placeholder(
        task.description
            .as_deref()
            .unwrap_or("Add task description..."),
    )
    .required(false);

    let datetime_placeholder = if task.recurrence.is_some() {
        if let Some(Recurrence::Weekly { days, hour, minute }) = &task.recurrence {
            format_days_and_time(days, *hour, *minute, timezone_service, &user_timezone)
        } else {
            "Enter days and hour (Mon,Wed,Fri 14:00)".to_string()
        }
    } else if let Some(utc_time) = task.scheduled_time {
        match timezone_service.format_from_utc_with_timezone(utc_time, &user_timezone) {
            Ok(local_time) => local_time,
            Err(_) => utc_time.format("%Y-%m-%d %H:%M").to_string(),
        }
    } else {
        "YYYY-MM-DD HH:MM".to_string()
    };

    let datetime_input =
        CreateInputText::new(InputTextStyle::Short, "New date and hour", "new_datetime")
            .placeholder(&datetime_placeholder)
            .required(false);

    let modal_id = format!("edit_task_modal_{}", task.id);

    let modal = CreateModal::new(&modal_id, "Edit task").components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(description_input),
        CreateActionRow::InputText(datetime_input),
    ]);

    let _ = interaction
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await;
}

/// process modal submission to edit the task
pub async fn process_edit_task_modal(
    ctx: &Context,
    modal: &ModalInteraction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !modal.data.custom_id.starts_with("edit_task_modal_") {
        return Ok(());
    }

    let task_id: u64 = modal
        .data
        .custom_id
        .strip_prefix("edit_task_modal_")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "Invalid task ID".to_string())?;

    let user_id = modal.user.id.get();

    // extract modal inputs
    let mut new_title: Option<String> = None;
    let mut new_description: Option<String> = None;
    let mut new_datetime_input: Option<String> = None;

    for row in &modal.data.components {
        for c in &row.components {
            if let ActionRowComponent::InputText(input) = c {
                match input.custom_id.as_str() {
                    "new_title" => {
                        if let Some(val) = &input.value {
                            if !val.trim().is_empty() {
                                new_title = Some(val.clone());
                            }
                        }
                    }
                    "new_description" => {
                        if let Some(desc) = &input.value {
                            if desc.trim().is_empty() {
                                new_description = Some("".to_string());
                            } else {
                                new_description = Some(desc.clone());
                            }
                        }
                    }
                    "new_datetime" => {
                        if let Some(dt_str) = &input.value {
                            if !dt_str.trim().is_empty() {
                                new_datetime_input = Some(dt_str.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // determine if it's a weekly task by checking the original task
    let is_weekly_task =
        if let Some(original_task) = task_service.get_task_for_editing(task_id, user_id).await {
            original_task.recurrence.is_some()
        } else {
            let _ = modal
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("❌ Couldn't find the task."),
                    ),
                )
                .await;
            return Ok(());
        };

    match task_service
        .edit_task(
            task_id,
            user_id,
            new_title,
            new_description,
            new_datetime_input,
            is_weekly_task,
            timezone_service.clone(),
        )
        .await
    {
        Ok(updated_task) => {
            let user_timezone = match timezone_service.get_user_timezone(user_id).await {
                Ok(Some(tz)) => tz,
                _ => "UTC".to_string(),
            };

            let date_str = format_task_date(&updated_task, timezone_service, &user_timezone);

            let embed = CreateEmbed::new()
                .title("Task Updated Successfully")
                .description(format!("Task **#{}** has been updated", updated_task.id))
                .field("Title", &updated_task.title, false)
                .field("Date", &date_str, false)
                .color(serenity::model::colour::Colour::DARK_GREEN);

            // Agregar campo de descripción solo si existe
            let embed = if let Some(desc) = &updated_task.description {
                if !desc.trim().is_empty() {
                    embed.field("Description", desc, false)
                } else {
                    embed
                }
            } else {
                embed
            };

            let _ = modal
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default().embed(embed),
                    ),
                )
                .await;
        }
        Err(error) => {
            let _ = modal
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content(format!("❌ Error editing task: {}", error)),
                    ),
                )
                .await;
        }
    }

    Ok(())
}
