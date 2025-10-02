use crate::application::commands::{
    register_add_task_command, register_help_command, register_list_tasks_command,
    register_remove_task_command, run_add_task, run_help_command, run_list_tasks, run_remove_task,
};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;
use serenity::all::{CommandInteraction, Context, CreateCommand, Interaction, Ready};
use serenity::async_trait;
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler {
    pub task_repo: Arc<TaskRepository>,
}

#[async_trait]
impl EventHandler for CommandHandler {
    // Ejecutado cada vez que alguien usa un comando slash
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        println!("Received interaction: {:?}", interaction.kind());

        if let Interaction::Command(command) = interaction {
            // aquí `command` es un CommandInteraction
            println!("Interaction received: {}", command.data.name);

            match command.data.name.as_str() {
                "help" => {
                    println!("Running /help command");
                    run_help_command(&ctx, &command).await;
                }
                "add_task" => {
                    println!("Running /add_task command");
                    run_add_task(&ctx, &command, &self.task_repo).await;
                }
                "list_tasks" => {
                    println!("Running /list_tasks command");
                    run_list_tasks(&ctx, &command, &self.task_repo).await;
                }
                "remove_task" => {
                    println!("Running /remove_task command");
                    run_remove_task(&ctx, &command, &self.task_repo).await;
                }
                _ => println!("Command not recognized: {}", command.data.name),
            }
        }
    }

    // Ejecutado cuando el bot se conecta
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        use serenity::builder::CreateCommand;
        use serenity::model::id::GuildId;

        let guild_id = GuildId::new(1422605167580155914);

        // Registrar todos los comandos
        println!("Registering commands...");
        let commands: Vec<CreateCommand> = vec![
            register_help_command(),
            register_add_task_command(),
            register_list_tasks_command(),
            register_remove_task_command(),
        ];

        match guild_id.set_commands(&ctx.http, commands).await {
            Ok(_) => println!("Commands registered successfully"),
            Err(err) => eprintln!("Error trying to register commands: {}", err),
        }

        // Iniciar scheduler en background
        start_scheduler(Arc::new(ctx), self.task_repo.clone());
        println!("Scheduler started");
    }
}
