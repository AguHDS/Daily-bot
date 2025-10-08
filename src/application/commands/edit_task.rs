use crate::application::services::task_service::TaskService;
use crate::domain::entities::task::Recurrence;
use serenity::all::{
    ActionRowComponent, CommandInteraction, ComponentInteraction, ComponentInteractionDataKind,
    Context, CreateActionRow, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, InputTextStyle, ModalInteraction,
};
use serenity::builder::{
    CreateInputText, CreateModal, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use std::sync::Arc;

/// Register /edit_task command
pub fn register_edit_task_command() -> CreateCommand {
    CreateCommand::new("edit_task").description("Edit your task")
}

/// Run /edit_task, show select menus for single and weekly tasks
pub async fn run_edit_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_service: &Arc<TaskService>,
) {
    let user_id = command.user.id.get();

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
                let label = task_service.format_task_for_display(task);
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
                let label = task_service.format_task_for_display(task);
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
        .placeholder(&task.message)
        .required(false);

    // delegate to TaskService for placeholder generation
    let datetime_placeholder = task_service.get_datetime_placeholder(&task);

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

/// process modal submission to edit the task
pub async fn process_edit_task_modal(
    ctx: &Context,
    modal: &ModalInteraction,
    task_service: &Arc<TaskService>,
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

    // delegate to TaskService for business logic
    match task_service
        .edit_task(
            task_id,
            user_id,
            new_title,
            new_datetime_input,
            is_weekly_task,
        )
        .await
    {
        Ok(updated_task) => {
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
