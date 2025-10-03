use crate::application::repositories::task_repository::TaskRepository;
use crate::application::domain::Recurrence;
use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::prelude::*;
use serenity::prelude::*;

// Registers /list_tasks command
pub fn register_list_tasks_command() -> CreateCommand {
    CreateCommand::new("list_tasks").description("Current task list")
}

// /list_tasks command logic
pub async fn run_list_tasks(
    ctx: &Context,
    command: &CommandInteraction,
    task_repo: &TaskRepository,
) {
    let tasks = task_repo.list_tasks();
    let user_id: u64 = command.user.id.into();

    // separate single and recurrent tasks
    let mut single_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_none())
        .collect();

    let mut recurrent_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_some())
        .collect();

    // order by ascending id
    single_tasks.sort_by_key(|t| t.id);
    recurrent_tasks.sort_by_key(|t| t.id);

    let mut content = String::from("📝 **Your tasks:**\n\n");

    // show single tasks first
    if !single_tasks.is_empty() {
        content.push_str("**Single Tasks:**\n");
        for task in single_tasks {
            let scheduled_str = task.scheduled_time
                .map_or("Not scheduled".to_string(), |dt| dt.format("%Y-%m-%d at %H:%M").to_string());
            content.push_str(&format!("**#{}**: {} — {}\n", task.id, task.message, scheduled_str));
        }
        content.push('\n');
    }

    // show recurrent tasks
    if !recurrent_tasks.is_empty() {
        content.push_str("**Weekly:**\n");
        for task in recurrent_tasks {
            let recurrence_str = match &task.recurrence {
                Some(Recurrence::Weekly { days, hour, minute }) => {
                    let days_str: Vec<String> = days.iter().map(|d| format!("{:?}", d)).collect();
                    format!("{} at {:02}:{:02}", days_str.join(", "), hour, minute)
                }
                Some(Recurrence::EveryXDays { interval, hour, minute }) => {
                    format!("Every {} days at {:02}:{:02}", interval, hour, minute)
                }
                _ => "Unknown recurrence".to_string(),
            };

            content.push_str(&format!("**#{}**: {} — {}\n", task.id, task.message, recurrence_str));
        }
    }

    if content.trim() == "📝 **Your tasks:**" {
        content = "You don't have any tasks.".to_string();
    }

    let builder = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::default().content(content),
    );

    let _ = command.create_response(&ctx.http, builder).await;
}
