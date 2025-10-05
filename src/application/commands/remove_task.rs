use crate::application::repositories::task_repository::TaskRepository;
use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use std::sync::Arc;

pub fn register_remove_task_command() -> CreateCommand {
    CreateCommand::new("remove_task").description("Task removal")
}

pub async fn run_remove_task(
    ctx: &Context,
    command: &CommandInteraction,
    repo: &Arc<dyn TaskRepository>,
) {
    let user_id = command.user.id.get();
    let tasks = repo.list_tasks();

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
                        .content("You don't have any task to delete")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let mut components: Vec<CreateActionRow> = Vec::new();

    // single tasks select menu
    if !single_tasks.is_empty() {
        let options: Vec<_> = single_tasks
            .iter()
            .map(|task| {
                let label = format!("#{}: {}", task.id, task.message);
                CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect();

        let select = CreateSelectMenu::new(
            "remove_menu_single",
            CreateSelectMenuKind::String { options },
        )
        .placeholder("Single tasks")
        .min_values(1)
        .max_values(1);

        components.push(CreateActionRow::SelectMenu(select));
    }

    // weekly tasks select menu
    if !weekly_tasks.is_empty() {
        let options: Vec<_> = weekly_tasks
            .iter()
            .map(|task| {
                let label = format!("#{}: {}", task.id, task.message);
                CreateSelectMenuOption::new(label, task.id.to_string())
            })
            .collect();

        let select = CreateSelectMenu::new(
            "remove_menu_weekly",
            CreateSelectMenuKind::String { options },
        )
        .placeholder("Weekly tasks")
        .min_values(1)
        .max_values(1);

        components.push(CreateActionRow::SelectMenu(select));
    }

    // remove all button
    let remove_all_button = CreateButton::new("remove_all_button")
        .label("🗑️ Delete all tasks")
        .style(ButtonStyle::Danger);
    components.push(CreateActionRow::Buttons(vec![remove_all_button]));

    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default()
                    .content("Select a task to delete:")
                    .components(components)
                    .ephemeral(true),
            ),
        )
        .await;
}

// handler for component interactions
pub async fn handle_remove_select(
    ctx: &Context,
    interaction: &ComponentInteraction,
    repo: &Arc<dyn TaskRepository>,
) {
    use serenity::all::{
        ButtonStyle, ComponentInteractionDataKind, CreateActionRow, CreateButton,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    };

    match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            if let Some(selected) = values.first() {
                if selected == "remove_all" {
                    let user_id = interaction.user.id.get();
                    let count = repo.remove_all_by_user(user_id);
                    let content = format!("✅ {} tasks deleted", count);
                    let _ = interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::default()
                                    .content(content)
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                    return;
                }

                match selected.parse::<u64>() {
                    Ok(task_id) => {
                        let removed = repo.remove_task(task_id);
                        let content = if removed {
                            format!("✅ Task {} deleted.", task_id)
                        } else {
                            format!("❌ Couldn't find task {}.", task_id)
                        };
                        let _ = interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::default()
                                        .content(content)
                                        .ephemeral(true),
                                ),
                            )
                            .await;
                    }
                    Err(_) => {
                        let _ = interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::default()
                                        .content("Invalid selection (couldn't parse task ID)")
                                        .ephemeral(true),
                                ),
                            )
                            .await;
                    }
                }
            }
        }

        ComponentInteractionDataKind::Button { .. } => match interaction.data.custom_id.as_str() {
            "remove_all_button" => {
                let confirm_yes = CreateButton::new("confirm_remove_all_yes")
                    .label("✅ Yes")
                    .style(ButtonStyle::Danger);
                let confirm_no = CreateButton::new("confirm_remove_all_no")
                    .label("❌ No")
                    .style(ButtonStyle::Secondary);

                let rows = vec![CreateActionRow::Buttons(vec![confirm_yes, confirm_no])];

                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("⚠️ Are you sure you want to delete all your tasks?")
                                .components(rows)
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
            "confirm_remove_all_yes" => {
                let user_id = interaction.user.id.get();
                let count = repo.remove_all_by_user(user_id);

                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content(format!("✅ {} tasks deleted successfully", count))
                                .components(vec![])
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
            "confirm_remove_all_no" => {
                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("❌ Operation cancelled")
                                .components(vec![])
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
            _ => {}
        },

        _ => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content("Interaction type not handled")
                            .ephemeral(true),
                    ),
                )
                .await;
        }
    }
}
