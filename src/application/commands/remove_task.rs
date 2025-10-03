use crate::application::repositories::task_repository::TaskRepository;
use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};

pub fn register_remove_task_command() -> CreateCommand {
    CreateCommand::new("remove_task").description("Interactive task removal")
}

pub async fn run_remove_task(
    ctx: &Context,
    command: &CommandInteraction,
    task_repo: &TaskRepository,
) {
    let user_id: u64 = u64::from(command.user.id);
    let tasks = task_repo.list_tasks();

    let single_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_none())
        .collect();

    let weekly_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.user_id == user_id && t.recurrence.is_some())
        .collect();

    println!(
        "DEBUG run_remove_task: user_id={} single={} weekly={}",
        user_id,
        single_tasks.len(),
        weekly_tasks.len()
    );

    if single_tasks.is_empty() && weekly_tasks.is_empty() {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .content("You don't have any task to delete."),
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

    // remove all tasks button
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
                    .components(components),
            ),
        )
        .await;
}

// handler for component interactions
pub async fn handle_remove_select(
    ctx: &Context,
    interaction: &ComponentInteraction,
    task_repo: &TaskRepository,
) {
    use serenity::all::{
        ButtonStyle, ComponentInteractionDataKind, CreateActionRow, CreateButton,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    };

    match &interaction.data.kind {
        // Select menus
        ComponentInteractionDataKind::StringSelect { values } => {
            if let Some(selected) = values.first() {
                if selected == "remove_all" {
                    let user_id = interaction.user.id.get();
                    let count = task_repo.remove_all_by_user(user_id);
                    let content = format!("✅ {} tasks deleted.", count);
                    let _ = interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::default().content(content),
                            ),
                        )
                        .await;
                    return;
                }

                match selected.parse::<u64>() {
                    Ok(task_id) => {
                        let removed = task_repo.remove_task(task_id);
                        let content = if removed {
                            format!("✅ Task {} deleted.", task_id)
                        } else {
                            format!("❌ Couldn't find a task {}.", task_id)
                        };

                        let _ = interaction
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::default().content(content),
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
                                        .content("Invalid selection (couldn't parse task Id.)"),
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
                                .components(rows),
                        ),
                    )
                    .await;
            }
            "confirm_remove_all_yes" => {
                let user_id = interaction.user.id.get();
                let count = task_repo.remove_all_by_user(user_id);

                let _ = interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content(format!("✅ {} tasks deleted successfully.", count))
                                .components(vec![]),
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
                                .content("❌ Operation cancelled.")
                                .components(vec![]),
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
                            .content("Interaction type not handled."),
                    ),
                )
                .await;
        }
    }
}
