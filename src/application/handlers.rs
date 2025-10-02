use crate::application::commands::help::{register_help_command, run_help_command};
use crate::application::repositories::task_repository::TaskRepository;
use crate::application::scheduler::scheduler_tokio::start_scheduler;
use serenity::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::sync::Arc;

pub struct CommandHandler;

#[async_trait]
impl EventHandler for CommandHandler {
    // Ejecutado cada vez que alguien usa un comando slash
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        use serenity::model::application::Interaction;

        if let Interaction::Command(ref command) = interaction {
            println!("Interaction received: {}", command.data.name);
            match command.data.name.as_str() {
                "help" => {
                    println!("Running /help command");
                    run_help_command(&ctx, command).await;
                }
                _ => println!("Command not registered: {}", command.data.name),
            }
        }
    }

    // Ejecutado cuando el bot se conecta, registrar comandos y arrancar scheduler
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot ready as {}", ready.user.name);

        use serenity::model::id::GuildId;

        // Debug: antes de crear el GuildId
        println!("Creating GuildId...");
        let guild_id = GuildId::new(1422605167580155914);
        println!("GuildId created: {}", guild_id.get());

        // Registrar comandos
        println!("Registering commands...");
        let commands: Vec<CreateCommand> = vec![register_help_command()];
        match guild_id.set_commands(&ctx.http, commands).await {
            Ok(_) => println!("Commands registered successfully"),
            Err(err) => eprintln!("Error trying to register commands: {}", err),
        }

        // Inicializar repositorio de tareas
        println!("Initializing TaskRepository...");
        let task_repo = Arc::new(TaskRepository::new());

        // Arrancar scheduler en segundo plano
        println!("Starting scheduler...");
        start_scheduler(Arc::new(ctx), task_repo.clone());
        println!("Scheduler started");
    }
}
