use crate::application::commands::utils::weekly_parser::parse_weekly_input;
use crate::application::domain::Recurrence;
use crate::application::repositories::task_repository::TaskRepository;
use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone, Timelike, Utc};
use serenity::all::{
    ActionRowComponent, CommandInteraction, ComponentInteraction, ComponentInteractionDataKind,
    Context, CreateActionRow, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, InputTextStyle, ModalInteraction,
};
use serenity::builder::{CreateInputText, CreateModal};
use std::sync::Arc;

pub fn register_edit_task_command() -> CreateCommand {
    CreateCommand::new("edit_task").description("Edit your task")
}

pub async fn run_edit_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_repo: &dyn TaskRepository,
) {
    let user_id = command.user.id.get();
    let tasks = task_repo.list_tasks();

    let single_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_none())
        .collect();
    let weekly_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_some())
        .collect();

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
                let label = format!("#{}: {}", task.id, task.message);
                serenity::all::CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect::<Vec<_>>();

        let select = serenity::all::CreateSelectMenu::new(
            "edit_menu_single",
            serenity::all::CreateSelectMenuKind::String { options },
        )
        .placeholder("Single tasks")
        .min_values(1)
        .max_values(1);

        components.push(CreateActionRow::SelectMenu(select));
    }

    if !weekly_tasks.is_empty() {
        let options = weekly_tasks
            .iter()
            .map(|task| {
                let label = format!("#{}: {}", task.id, task.message);
                serenity::all::CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect::<Vec<_>>();

        let select = serenity::all::CreateSelectMenu::new(
            "edit_menu_weekly",
            serenity::all::CreateSelectMenuKind::String { options },
        )
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
    task_repo: &dyn TaskRepository,
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
                println!("No value selected in select menu");
                return;
            }
        }
        _ => {
            println!("Component is not a string select menu");
            return;
        }
    };

    let task = match task_repo
        .list_tasks()
        .into_iter()
        .find(|t| t.id == selected_id)
    {
        Some(t) => t,
        None => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    serenity::all::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("❌ Couldn't find the task."),
                    ),
                )
                .await;
            return;
        }
    };

    let title_input = CreateInputText::new(InputTextStyle::Short, "New title", "new_title")
        .placeholder(&task.message)
        .required(false);

    let datetime_placeholder = if task.recurrence.is_some() {
        "Enter days and hour (Mon,Wed,Fri 14:00)".to_string()
    } else {
        task.scheduled_time
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "YYYY-MM-DD HH:MM".to_string())
    };

    let datetime_input =
        CreateInputText::new(InputTextStyle::Short, "New date and hour", "new_datetime")
            .placeholder(&datetime_placeholder)
            .required(false);

    let modal_id = format!("edit_task_modal_{}", task.id);

    let modal = CreateModal::new(&modal_id, "Edit task").components(vec![
        CreateActionRow::InputText(title_input),
        CreateActionRow::InputText(datetime_input),
    ]);

    let _ = interaction
        .create_response(&ctx.http, CreateInteractionResponse::Modal(modal))
        .await;
}

/// Process modal submission to edit the task
pub async fn process_edit_task_modal(
    ctx: &Context,
    modal: &ModalInteraction,
    repo: &Arc<dyn TaskRepository>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !modal.data.custom_id.starts_with("edit_task_modal_") {
        return Ok(());
    }

    let task_id: u64 = modal
        .data
        .custom_id
        .strip_prefix("edit_task_modal_")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            "Invalid task ID"
        })?;

    let task = match repo.list_tasks().into_iter().find(|t| t.id == task_id) {
        Some(t) => t,
        None => {
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
        }
    };

    let mut new_title: Option<String> = None;
    let mut new_datetime: Option<DateTime<Utc>> = None;
    let mut new_recurrence: Option<Recurrence> = None;

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
                    "new_datetime" => {
                        if let Some(dt_str) = &input.value {
                            if !dt_str.trim().is_empty() {
                                if task.recurrence.is_some() {
                                    let (days, hour, minute, _) = parse_weekly_input(dt_str)
                                        .map_err(
                                            |e| -> Box<dyn std::error::Error + Send + Sync> {
                                                Box::new(std::io::Error::new(
                                                    std::io::ErrorKind::Other,
                                                    e.to_string(),
                                                ))
                                            },
                                        )?;

                                    let mut first_time = Utc::now();
                                    while !days.contains(&first_time.weekday()) {
                                        first_time = first_time + chrono::Duration::days(1);
                                    }
                                    first_time = first_time
                                        .with_hour(hour as u32)
                                        .and_then(|t| t.with_minute(minute as u32))
                                        .unwrap_or(first_time);

                                    if first_time < Utc::now() {
                                        let builder = CreateInteractionResponseMessage::default()
                                            .content("❌ Cannot set a weekly task in the past.")
                                            .ephemeral(true);
                                        modal
                                            .create_response(
                                                ctx,
                                                CreateInteractionResponse::Message(builder),
                                            )
                                            .await?;
                                        return Ok(());
                                    }

                                    new_datetime = Some(first_time);
                                    new_recurrence =
                                        Some(Recurrence::Weekly { days, hour, minute });
                                } else {
                                    let naive =
                                        NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M")
                                            .map_err(|_| "❌ Invalid date. Use YYYY-MM-DD HH:MM")?;
                                    let parsed_dt = Utc.from_utc_datetime(&naive);

                                    if parsed_dt < Utc::now() {
                                        let builder = CreateInteractionResponseMessage::default()
                                            .content("❌ You can't set a date in past time.")
                                            .ephemeral(true);
                                        modal
                                            .create_response(
                                                ctx,
                                                CreateInteractionResponse::Message(builder),
                                            )
                                            .await?;
                                        return Ok(());
                                    }

                                    new_datetime = Some(parsed_dt);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let result = repo.edit_task(task_id, new_title, new_datetime.clone(), new_recurrence);
    if let Err(err) = &result {
        let _ = modal
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content(format!("❌ Error editing task: {}", err)),
                ),
            )
            .await;
        return Ok(());
    }

    if let Ok(updated_task) = result {
        let date_str =
            if let Some(Recurrence::Weekly { days, hour, minute }) = updated_task.recurrence {
                let days_str = days
                    .iter()
                    .map(|d| format!("{:?}", d))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{} at {:02}:{:02}", days_str, hour, minute)
            } else if let Some(dt) = updated_task.scheduled_time {
                dt.format("%Y-%m-%d %H:%M").to_string()
            } else {
                "Date missing".to_string()
            };

        let content = format!(
            "✅ Task **#{}** updated:\n**Title:** {}\n**Date:** {}",
            updated_task.id, updated_task.message, date_str
        );

        let _ = modal
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default().content(content),
                ),
            )
            .await;
    }

    Ok(())
}
