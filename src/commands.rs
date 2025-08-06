use crate::database::TaskManager;
use crate::display::{StatusDisplay, format_task_line_with_number, print_task_details};
use crate::editor;
use crate::types::{EditField, Status, StatusFilter, Task, TaskCommand, TaskError};
use crate::utils::{
    delete_database, format_task_not_found_message, is_number, parse_due_date, resolve_task,
    save_last_list_all, status_filter_from_params, was_last_list_all,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use colored::*;
use std::io::{self, Write};
use terminal_size::{Width, terminal_size};

const WRAP_COLUMN: usize = 80;
const SHORT_ID_LENGTH: usize = 8;

pub fn execute_command(manager: &TaskManager, command: TaskCommand) -> Result<(), TaskError> {
    match command {
        TaskCommand::Add {
            name,
            description,
            due_date,
        } => handle_add(manager, name, description, due_date),

        TaskCommand::Completions {
            shell,
            dynamic_bash,
            dynamic_fish,
        } => handle_completions(shell, dynamic_bash, dynamic_fish),

        TaskCommand::Delete {
            id_or_index,
            status,
        } => handle_delete(manager, id_or_index, status),

        TaskCommand::List {
            status,
            show_all,
            show_descriptions,
        } => handle_list(manager, status, show_all, show_descriptions),

        TaskCommand::ListNames { show_all, status } => handle_list_names(manager, show_all, status),

        TaskCommand::Show { id } => handle_show(manager, id),

        TaskCommand::ShowName {
            id_or_index,
            status,
        } => handle_show_name(manager, id_or_index, status),

        TaskCommand::Edit { id_or_index, field } => handle_edit(manager, id_or_index, field),

        TaskCommand::UpdateStatus { id, status } => handle_update_status(manager, id, status),

        TaskCommand::DeleteDatabase => delete_database(),

        TaskCommand::Ids { short_only, filter } => handle_ids(manager, short_only, filter),

        TaskCommand::EditWithEditor { id_or_index } => {
            handle_edit_with_editor(manager, id_or_index)
        }
    }
}

fn handle_add(
    manager: &TaskManager,
    name: String,
    description: Option<String>,
    due_date: Option<DateTime<Utc>>,
) -> Result<(), TaskError> {
    let task = Task::new(name, description, due_date)?;
    manager.add_task(task.clone())?;
    println!("{}", "task created successfully".bright_green());
    print_task_details(&task, true);
    Ok(())
}

fn handle_completions(
    shell: String,
    dynamic_bash: String,
    dynamic_fish: String,
) -> Result<(), TaskError> {
    let mut cmd = crate::cli::build_cli();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    match shell.as_str() {
        "bash" => {
            generate(Bash, &mut cmd, "tarea", &mut out);
            writeln!(out, "{}", dynamic_bash)?;
        }
        "zsh" => {
            generate(Zsh, &mut cmd, "tarea", &mut io::stdout());
        }
        "fish" => {
            generate(Fish, &mut cmd, "tarea", &mut io::stdout());
            writeln!(out, "{}", dynamic_fish)?;
        }
        "powershell" => generate(PowerShell, &mut cmd, "tarea", &mut io::stdout()),
        "elvish" => generate(Elvish, &mut cmd, "tarea", &mut io::stdout()),
        _ => unreachable!(),
    };
    Ok(())
}

fn handle_delete(
    manager: &TaskManager,
    id_or_index: String,
    status: Option<Status>,
) -> Result<(), TaskError> {
    let use_all = was_last_list_all();
    let filter = match (status.clone(), use_all) {
        (Some(_s), _) if use_all => StatusFilter::All,
        (Some(s), _) => StatusFilter::AnyOf(vec![s]),
        (None, true) => StatusFilter::All,
        (None, false) => StatusFilter::PendingOnly,
    };
    let task_list = manager.list_tasks(filter)?;

    let task_opt = if is_number(&id_or_index) {
        let idx: usize = id_or_index.parse().unwrap_or(0);
        task_list.into_iter().nth(idx.saturating_sub(1))
    } else {
        task_list
            .into_iter()
            .find(|t| t.id.starts_with(&id_or_index))
    };

    match task_opt {
        Some(task) => {
            let task_display = if task.name.len() > 50 {
                format!("{}...", &task.name[..47])
            } else {
                task.name.clone()
            };

            let confirmed = {
                print!("delete task '{}'? (y/N): ", task_display);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
            };

            if confirmed {
                if manager.delete_task_by_id(&task.id)? {
                    println!("{}", "success".bright_green());
                    println!();
                    handle_list(manager, status, use_all, false)?;
                } else {
                    println!(
                        "{}",
                        "task not found (may have been already deleted)".bright_red()
                    );
                }
            } else {
                println!("{}", "task deletion cancelled".bright_yellow());
            }
        }
        None => {
            let context = status
                .map(|s| format!(" in {} tasks", s))
                .unwrap_or_else(|| {
                    if was_last_list_all() {
                        " in all tasks".to_string()
                    } else {
                        " in pending tasks".to_string()
                    }
                });

            println!(
                "{}",
                format_task_not_found_message(&id_or_index, Some(&context))
            );
        }
    }
    Ok(())
}

fn handle_list(
    manager: &TaskManager,
    status: Option<Status>,
    show_all: bool,
    show_descriptions: bool,
) -> Result<(), TaskError> {
    let filter = status_filter_from_params(status.clone(), show_all);
    let tasks = manager.list_tasks(filter)?;

    if tasks.is_empty() {
        let message = match (show_all, status) {
            (true, _) => "no tasks found".to_string(),
            (false, Some(s)) => format!("no {} tasks found", s),
            (false, None) => "no pending tasks found".to_string(),
        };
        println!("{}", message.dimmed());
        return Ok(());
    }

    let layout = calculate_list_layout(&tasks, show_descriptions);

    for (idx, task) in tasks.iter().enumerate() {
        format_task_line_with_number(
            idx + 1,
            layout.number_width,
            task,
            layout.name_width,
            layout.time_width,
            layout.indent_len,
            layout.time_col_start,
            show_descriptions,
            StatusDisplay::Dot,
        );
    }
    save_last_list_all(show_all)?;
    Ok(())
}

fn handle_list_names(
    manager: &TaskManager,
    show_all: bool,
    status: Option<Status>,
) -> Result<(), TaskError> {
    let filter = status_filter_from_params(status, show_all);
    let tasks = manager.list_tasks(filter)?;
    if tasks.is_empty() {
        println!("{}", "no tasks found".dimmed());
    } else {
        for (idx, t) in tasks.iter().enumerate() {
            println!("{:>3}. {}", idx + 1, t.name);
        }
    }
    Ok(())
}

fn handle_show(manager: &TaskManager, id: String) -> Result<(), TaskError> {
    let use_all = was_last_list_all();
    let task_opt = resolve_task(manager, &id, use_all)?;

    match task_opt {
        Some(task) => print_task_details(&task, false),
        None => println!("{}", format_task_not_found_message(&id, None)),
    }
    Ok(())
}

fn handle_show_name(
    manager: &TaskManager,
    id_or_index: String,
    status: Option<Status>,
) -> Result<(), TaskError> {
    let use_all = was_last_list_all();
    let filter = status_filter_from_params(status.clone(), use_all);
    let task_list = manager.list_tasks(filter)?;
    let task_opt = if is_number(&id_or_index) {
        let idx: usize = id_or_index.parse().unwrap_or(0);
        task_list.into_iter().nth(idx.saturating_sub(1))
    } else {
        task_list
            .into_iter()
            .find(|t| t.id.starts_with(&id_or_index))
    };
    match task_opt {
        Some(t) => println!("{}", t.name),
        None => {
            let context = status.map(|s| format!(" in {} tasks", s));
            println!(
                "{}",
                format_task_not_found_message(&id_or_index, context.as_deref())
            );
        }
    }
    Ok(())
}

fn handle_edit(
    manager: &TaskManager,
    id_or_index: String,
    field: EditField,
) -> Result<(), TaskError> {
    let use_all = was_last_list_all();
    let full_id = match resolve_task(manager, &id_or_index, use_all)? {
        Some(t) => t.id,
        None => {
            println!("{}", format_task_not_found_message(&id_or_index, None));
            return Ok(());
        }
    };

    let changed = match field {
        EditField::Name(n) => manager.update_name(&full_id, &n)?,
        EditField::Description(d) => manager.update_description(&full_id, &d)?,
        EditField::DueDate(dt) => manager.update_due(&full_id, Some(dt))?,
    };

    if changed {
        println!("{}", "task updated".bright_green());
    } else {
        println!("{}", "nothing changed".bright_yellow());
    }
    Ok(())
}

fn handle_update_status(
    manager: &TaskManager,
    id: String,
    status: Status,
) -> Result<(), TaskError> {
    let target_id = match resolve_task(manager, &id, was_last_list_all())? {
        Some(t) => t.id,
        None => {
            println!("{}", format_task_not_found_message(&id, None));
            return Ok(());
        }
    };

    match manager.update_task_status(&target_id, status.clone())? {
        true => {
            let color = match status {
                Status::Done => "green",
                Status::Pending => "yellow",
                Status::Standby => "blue",
            };
            println!(
                "{}",
                format!("Task {} marked as {}", id, status).color(color)
            );
        }
        false => println!("{}", format_task_not_found_message(&id, None)),
    }
    Ok(())
}

fn handle_ids(
    manager: &TaskManager,
    short_only: bool,
    filter: Vec<Status>,
) -> Result<(), TaskError> {
    let tasks = manager.list_tasks(StatusFilter::AnyOf(filter))?;

    for task in tasks {
        let out = if short_only {
            &task.id[..SHORT_ID_LENGTH]
        } else {
            &task.id
        };
        println!("{out}");
    }
    Ok(())
}

fn handle_edit_with_editor(manager: &TaskManager, id_or_index: String) -> Result<(), TaskError> {
    let use_all = was_last_list_all();
    let task = match resolve_task(manager, &id_or_index, use_all)? {
        Some(t) => t,
        None => {
            println!("{}", format_task_not_found_message(&id_or_index, None));
            return Ok(());
        }
    };

    let edited = match editor::edit_via_editor(&task) {
        Ok(ed) => ed,
        Err(e) => {
            println!("{}", e);
            return Ok(());
        }
    };

    let mut changed = false;
    if edited.name.trim() != task.name {
        manager.update_name(&task.id, edited.name.trim())?;
        changed = true;
    }
    if edited.description != task.description {
        manager.update_description(&task.id, &edited.description)?;
        changed = true;
    }
    let new_due_date = match edited.due.as_deref() {
        Some(s) if !s.trim().is_empty() => match parse_due_date(s) {
            Ok(dt) => Some(dt),
            Err(e) => {
                println!("{}", e);
                return Ok(());
            }
        },
        _ => None,
    };
    if new_due_date != task.due_date {
        manager.update_due(&task.id, new_due_date)?;
        changed = true;
    }
    if changed {
        println!("{}", "task updated".bright_green());
    } else {
        println!("{}", "nothing changed".bright_yellow());
    }
    Ok(())
}

struct ListLayout {
    number_width: usize,
    name_width: usize,
    time_width: usize,
    indent_len: usize,
    time_col_start: usize,
}

fn calculate_list_layout(tasks: &[Task], show_descriptions: bool) -> ListLayout {
    let number_width = tasks.len().to_string().len();

    let created_width = tasks
        .iter()
        .map(|t| {
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str(&t.date, "%Y-%m-%d %H:%M:%S").unwrap(),
                Utc,
            );
            crate::display::pretty_time(dt).len()
        })
        .max()
        .unwrap_or(0);

    let max_due_extra = tasks
        .iter()
        .map(|t| {
            if t.status != Status::Done {
                t.due_date
                    .map(|d| 3 + crate::display::pretty_time(d).len() + 1)
                    .unwrap_or(0)
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0);

    let term = term_width();
    let base_cols = number_width + 2 + SHORT_ID_LENGTH + 1 + 1 + 1 + 1;
    let time_width = created_width;
    let cap = term
        .saturating_sub(base_cols + time_width + max_due_extra)
        .max(10);

    let longest_date_len = time_width + max_due_extra;
    let forced_total = WRAP_COLUMN + 1 + longest_date_len;

    let should_force_time_col =
        show_descriptions && base_cols < WRAP_COLUMN && term >= forced_total;

    let name_width = if should_force_time_col {
        WRAP_COLUMN + 2 - base_cols
    } else {
        tasks
            .iter()
            .map(|t| truncate_with_dots(&t.name, cap).len())
            .max()
            .unwrap_or(10)
            .max(10)
    };

    let indent_len = number_width + 2;
    let time_col_start = if should_force_time_col {
        WRAP_COLUMN
    } else {
        base_cols + name_width
    };

    ListLayout {
        number_width,
        name_width,
        time_width,
        indent_len,
        time_col_start,
    }
}

fn term_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

fn truncate_with_dots(s: &str, limit: usize) -> String {
    if s.len() <= limit {
        return s.to_string();
    }

    let truncated: String = s.chars().take(limit - 3).collect();
    format!("{}...", truncated)
}

pub fn estimated_lines(command: &TaskCommand, manager: &TaskManager) -> usize {
    match command {
        TaskCommand::List {
            show_descriptions,
            show_all,
            status,
        } => {
            let filter = status_filter_from_params(status.clone(), *show_all);
            if let Ok(tasks) = manager.list_tasks(filter) {
                if *show_descriptions {
                    tasks.len() * 4 // 1 title + 2 blanks + 1 wrapped line (avg)
                } else {
                    tasks.len() // exactly 1 line per task
                }
            } else {
                0
            }
        }
        _ => 0, // other commands never exceed one screen
    }
}
