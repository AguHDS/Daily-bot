use crate::application::services::config_service::ConfigService;
use crate::application::services::notification_service::NotificationService;
use crate::application::services::task_service::TaskService;
use crate::application::services::timezone_service::TimezoneService;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

/// Handle slash commands
pub async fn handle_command(
    ctx: &Context,
    interaction: &Interaction,
    task_service: &Arc<TaskService>,
    config_service: &Arc<ConfigService>,
    _notification_service: &Arc<NotificationService>,
    timezone_service: &Arc<TimezoneService>,
) {
    if let Some(command) = interaction.clone().command() {
        match command.data.name.as_str() {
            "list_tasks" => {
                crate::application::commands::list_tasks::run_list_tasks(
                    ctx,
                    &command,
                    task_service,
                    timezone_service,
                )
                .await;
            }
            "remove_task" => {
                crate::application::commands::remove_task::run_remove_task(
                    ctx,
                    &command,
                    task_service,
                )
                .await;
            }
            "help" => {
                crate::application::commands::help::run_help_command(ctx, &command).await;
            }
            "edit_task" => {
                crate::application::commands::edit_task::run_edit_task(
                    ctx,
                    &command,
                    task_service,
                    timezone_service,
                )
                .await;
            }
            "set_notification_channel" => {
                crate::application::commands::set_notification_channel::run_set_notification_channel(
                    ctx, &command, config_service,
                )
                .await;
            }
            "timezone" => {
                // the timezone command is already handled directly in bot.rs
                // this block should never be triggered
                println!("Timezone command should be handled directly in bot.rs");
            }
            _ => println!("Command not recognized: {}", command.data.name),
        }
    }
}

/// Handle components (buttons, select menus)
pub async fn handle_component(
    ctx: &Context,
    interaction: &Interaction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) {
    if let Some(component) = interaction.clone().message_component() {
        let custom_id = component.data.custom_id.as_str();

        // Handle timezone components first
        if custom_id.starts_with("timezone_confirm:") {
            let timezone_id = &custom_id["timezone_confirm:".len()..];
            crate::application::commands::timezone::handle_timezone_confirm(
                ctx,
                &component,
                timezone_id,
                timezone_service,
            )
            .await;
            return;
        }

        match custom_id {
            "timezone_select" => {
                crate::application::commands::timezone::handle_timezone_select(
                    ctx,
                    &component,
                    timezone_service,
                )
                .await;
                return;
            }
            "timezone_cancel" => {
                crate::application::commands::timezone::handle_timezone_cancel(ctx, &component)
                    .await;
                return;
            }
            _ => {} // continue with other handlers
        }

        // handle remove-related components
        let remove_ids = [
            "remove_menu_single",
            "remove_menu_weekly",
            "remove_all_button",
            "confirm_remove_all_yes",
            "confirm_remove_all_no",
        ];

        if remove_ids.contains(&custom_id) {
            crate::application::commands::remove_task::handle_remove_select(
                ctx,
                &component,
                task_service,
            )
            .await;
            return;
        }

        // handle edit-related components
        let edit_ids = ["edit_menu_single", "edit_menu_weekly"];
        if edit_ids.contains(&custom_id) {
            crate::application::commands::edit_task::handle_edit_select(
                ctx,
                &component,
                task_service,
                timezone_service, // ← NUEVO
            )
            .await;
            return;
        }

        // in this block, no handler was found
        println!("Unhandled component with custom_id: {}", custom_id);
    }
}

/// Handles modal submissions
pub async fn handle_modal(
    ctx: &Context,
    interaction: &Interaction,
    task_service: &Arc<TaskService>,
    timezone_service: &Arc<TimezoneService>,
) {
    if let Some(modal) = interaction.clone().modal_submit() {
        let custom_id = modal.data.custom_id.as_str();

        if custom_id.starts_with("edit_task_modal_") {
            crate::application::commands::edit_task::process_edit_task_modal(
                ctx,
                &modal,
                task_service,
                timezone_service, // ← NUEVO
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("Failed to process edit task modal: {}", err);
            });
        } else if custom_id.starts_with("single_task_modal")
            || custom_id.starts_with("weekly_task_modal")
        {
            crate::application::commands::add_task::process_task_modal_input(
                ctx,
                &modal,
                task_service,
                timezone_service,
            )
            .await
            .unwrap_or_else(|err| {
                eprintln!("Failed to process add task modal input: {}", err);
            });
        } else {
            println!("Ignoring modal with unrecognized custom_id: {}", custom_id);
        }
    }
}
