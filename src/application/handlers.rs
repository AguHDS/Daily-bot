use crate::application::commands::{
    run_add_task, run_help_command, run_list_tasks, run_remove_task,
};
use crate::application::repositories::task_repository::TaskRepository;
use serenity::all::{Context, Interaction};
use serenity::async_trait;
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_repo: Arc<TaskRepository>,
}

#[async_trait]
impl EventHandler for CommandHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // slash commands
        if let Interaction::Command(command) = &interaction {
            println!("Interaction received: {}", command.data.name);

            match command.data.name.as_str() {
                "help" => {
                    println!("Running /help command");
                    run_help_command(&ctx, command).await;
                }
                "add_task" => {
                    println!("Running /add_task command");
                    run_add_task(&ctx, command, &self.task_repo).await;
                }
                "list_tasks" => {
                    println!("Running /list_tasks command");
                    run_list_tasks(&ctx, command, &self.task_repo).await;
                }
                "remove_task" => {
                    println!("Running /remove_task command");
                    run_remove_task(&ctx, command, &self.task_repo).await;
                }
                _ => println!("Command not recognized: {}", command.data.name),
            }
        }

        // Modal submit (single & weekly)
        if let Interaction::Modal(modal) = &interaction {
            if modal.data.custom_id.starts_with("single_task_modal|") {
                let message = modal
                    .data
                    .custom_id
                    .strip_prefix("single_task_modal|")
                    .unwrap()
                    .to_string();

                if let Err(err) = crate::application::commands::add_task::process_single_task_input(
                    &ctx,
                    modal,
                    &self.task_repo,
                    message,
                )
                .await
                {
                    eprintln!("Error processing single task modal: {}", err);
                }
            } else if modal.data.custom_id.starts_with("weekly_task_modal|") {
                let message = modal
                    .data
                    .custom_id
                    .strip_prefix("weekly_task_modal|")
                    .unwrap()
                    .to_string();

                if let Err(err) = crate::application::commands::add_task::process_weekly_task_input(
                    &ctx,
                    modal,
                    &self.task_repo,
                    message,
                )
                .await
                {
                    eprintln!("Error processing weekly task modal: {}", err);
                }
            }
        }
    }
}
