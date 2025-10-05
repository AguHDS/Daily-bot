use crate::application::commands::{
    add_task, edit_task, remove_task, run_add_task, run_help_command, run_list_tasks,
    run_remove_task,
};
use crate::application::repositories::task_repository::TaskRepository;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

/// Handle slash commands
pub async fn handle_command(
    ctx: &Context,
    interaction: &Interaction,
    task_repo: &Arc<dyn TaskRepository>,
) {
    if let Some(command) = interaction.clone().command() {
        match command.data.name.as_str() {
            "add_task" => run_add_task(ctx, &command, task_repo).await,
            "list_tasks" => run_list_tasks(ctx, &command, task_repo).await,
            "remove_task" => run_remove_task(ctx, &command, task_repo).await,
            "help" => run_help_command(ctx, &command).await,
            "edit_task" => edit_task::run_edit_task(ctx, &command, task_repo.as_ref()).await,
            _ => println!("Command not recognized: {}", command.data.name),
        }
    }
}

/// Handle components (buttons, select menus)
pub async fn handle_component(
    ctx: &Context,
    interaction: &Interaction,
    task_repo: &Arc<dyn TaskRepository>,
) {
    if let Some(component) = interaction.clone().message_component() {
        let remove_ids = [
            "remove_menu_single",
            "remove_menu_weekly",
            "remove_all_button",
            "confirm_remove_all_yes",
            "confirm_remove_all_no",
        ];

        if remove_ids.contains(&component.data.custom_id.as_str()) {
            remove_task::handle_remove_select(ctx, &component, task_repo).await;
        }

        let edit_ids = ["edit_menu_single", "edit_menu_weekly"];
        if edit_ids.contains(&component.data.custom_id.as_str()) {
            edit_task::handle_edit_select(ctx, &component, task_repo.as_ref()).await;
        }
    }
}

/// Handles modal submissions
pub async fn handle_modal(
    ctx: &Context,
    interaction: &Interaction,
    task_repo: &Arc<dyn TaskRepository>,
) {
    if let Some(modal) = interaction.clone().modal_submit() {
        let custom_id = &modal.data.custom_id;

        if custom_id.starts_with("edit_task_modal_") {
            if let Err(err) = crate::application::commands::edit_task::process_edit_task_modal(
                ctx, &modal, task_repo,
            )
            .await
            {
                eprintln!("Failed to process edit task modal: {}", err);
            }
        } else if custom_id.starts_with("single_task_modal")
            || custom_id.starts_with("weekly_task_modal")
        {
            if let Err(err) = crate::application::commands::add_task::process_task_modal_input(
                ctx, &modal, task_repo,
            )
            .await
            {
                eprintln!("Failed to process add task modal input: {}", err);
            }
        } else {
            println!("Ignoring modal with unrecognized custom_id: {}", custom_id);
        }
    }
}
