use crate::application::commands::{
    register_add_task_command, register_help_command, register_list_tasks_command,
    register_remove_task_command, run_add_task, run_help_command, run_list_tasks, run_remove_task,
    edit_task,
};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;

use serenity::model::{application::Interaction, gateway::Ready, id::GuildId};
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_repo: Arc<TaskRepository>,
}

#[serenity::async_trait]
impl EventHandler for CommandHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        // register commands in each guild
        for guild_status in ready.guilds {
            let guild_id: GuildId = guild_status.id;

            let _ = guild_id
                .create_command(&ctx.http, register_add_task_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_list_tasks_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_remove_task_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, register_help_command())
                .await;
            let _ = guild_id
                .create_command(&ctx.http, edit_task::register_edit_task_command())
                .await;

            println!("Commands registered for guild {}", guild_id.get());
        }

        start_scheduler(Arc::new(ctx), self.task_repo.clone());
        println!("Scheduler started");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // handle slash commands
        if let Some(command) = interaction.clone().command() {
            println!("Received command interaction: {}", command.data.name);
            match command.data.name.as_str() {
                "add_task" => run_add_task(&ctx, &command, &self.task_repo).await,
                "list_tasks" => run_list_tasks(&ctx, &command, &self.task_repo).await,
                "remove_task" => run_remove_task(&ctx, &command, &self.task_repo).await,
                "help" => run_help_command(&ctx, &command).await,
                "edit_task" => edit_task::run_edit_task(&ctx, &command, &self.task_repo).await,
                _ => println!("Command not recognized: {}", command.data.name),
            }
        }

        // components management (select menus, buttons)
        if let Some(component) = interaction.clone().message_component() {
            let relevant_remove_ids = [
                "remove_menu_single",
                "remove_menu_weekly",
                "remove_all_button",
                "confirm_remove_all_yes",
                "confirm_remove_all_no",
            ];

            if relevant_remove_ids.contains(&component.data.custom_id.as_str()) {
                crate::application::commands::remove_task::handle_remove_select(
                    &ctx,
                    &component,
                    &self.task_repo,
                )
                .await;
            }

            let relevant_edit_ids = ["edit_menu_single", "edit_menu_weekly"];

            if relevant_edit_ids.contains(&component.data.custom_id.as_str()) {
                crate::application::commands::edit_task::handle_edit_select(
                    &ctx,
                    &component,
                    &self.task_repo,
                )
                .await;
            }
        }

        // handle submit modal
        if let Some(modal) = interaction.clone().modal_submit() {
            let custom_id = &modal.data.custom_id;

            // single task modal
            if custom_id.starts_with("single_task_modal|") {
                let parts: Vec<&str> = custom_id.splitn(2, '|').collect();
                let message = parts.get(1).unwrap_or(&"").to_string();

                if let Err(err) = crate::application::commands::add_task::process_single_task_input(
                    &ctx,
                    &modal,
                    &self.task_repo,
                    message,
                )
                .await
                {
                    eprintln!("Failed to process single task input: {}", err);
                }
            } else if custom_id.starts_with("weekly_task_modal|") {
                let parts: Vec<&str> = custom_id.splitn(2, '|').collect();
                let message = parts.get(1).unwrap_or(&"").to_string();

                if let Err(err) = crate::application::commands::add_task::process_weekly_task_input(
                    &ctx,
                    &modal,
                    &self.task_repo,
                    message,
                )
                .await
                {
                    eprintln!("Failed to process weekly task input: {}", err);
                }
            }
            else if custom_id.starts_with("edit_task_modal|") {
                if let Err(err) =
                    edit_task::process_edit_task_modal(&ctx, &modal, &self.task_repo).await
                {
                    eprintln!("Failed to process edit task modal: {}", err);
                }
            }
        }
    }
}

pub async fn run_bot() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected token in environment");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let task_repo = Arc::new(TaskRepository::new());
    let handler = CommandHandler {
        task_repo: task_repo.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    client.start().await?;
    Ok(())
}
