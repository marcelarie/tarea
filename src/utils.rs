use crate::database::TaskManager;
use crate::types::{Status, StatusFilter, Task, TaskError};
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};
use colored::*;
use std::io;
use std::path::PathBuf;
use std::{env, fs};

const MAX_TASK_NAME_LENGTH: usize = 120;

pub fn validate_task_name(name: &str) -> Result<(), TaskError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(TaskError::InvalidInput(
            "Task name cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > MAX_TASK_NAME_LENGTH {
        return Err(TaskError::InvalidInput(format!(
            "Task name too long (max {MAX_TASK_NAME_LENGTH} characters). \
          Put additional details in the description.",
        )));
    }

    Ok(())
}

pub fn parse_due_date(input: &str) -> Result<DateTime<Utc>, TaskError> {
    let trimmed = input.trim().to_lowercase();

    match trimmed.as_str() {
        "today" => {
            let date = Local::now().date_naive();
            return Ok(date
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
                .with_timezone(&Utc));
        }
        "tomorrow" => {
            let date = (Local::now() + Duration::days(1)).date_naive();
            return Ok(date
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap()
                .with_timezone(&Utc));
        }
        _ => {}
    }

    let cleaned = trimmed.replace(' ', "");

    if let Some(h_pos) = cleaned.find('h') {
        let (hours_part, rest) = cleaned.split_at(h_pos);
        let rest = &rest[1..]; // drop 'h'

        if let Ok(h) = hours_part.parse::<i64>() {
            let mut duration = Duration::hours(h);
            if !rest.is_empty() {
                if let Some(mins_str) = rest.strip_suffix('m') {
                    if !mins_str.is_empty() {
                        if let Ok(m) = mins_str.parse::<i64>() {
                            duration += Duration::minutes(m);
                        }
                    }
                } else {
                    return Err(TaskError::InvalidDate(format!(
                        "Unable to parse '{}'. Expected minutes after hours, e.g. '4h30m'",
                        input
                    )));
                }
            }
            return Ok(Utc::now() + duration);
        }
    }

    if let Some(mins_str) = cleaned.strip_suffix('m') {
        if let Ok(m) = mins_str.parse::<i64>() {
            return Ok(Utc::now() + Duration::minutes(m));
        }
    }

    for fmt in ["%Y-%m-%d", "%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&trimmed, fmt) {
            let local_dt = Local
                .from_local_datetime(&naive)
                .single()
                .ok_or_else(|| TaskError::InvalidDate("Ambiguous or invalid local time".into()))?;

            return Ok(local_dt.with_timezone(&Utc));
        }
    }

    Err(TaskError::InvalidDate(format!(
        "Unable to parse '{}'. Use natural language like 'today', '2h 30m', or an absolute date 'YYYY-MM-DD [HH:MM[:SS]]'",
        input
    )))
}

pub fn status_filter_from_params(status: Option<Status>, show_all: bool) -> StatusFilter {
    if show_all {
        StatusFilter::All
    } else {
        match status {
            Some(s) => StatusFilter::AnyOf(vec![s]),
            None => StatusFilter::PendingOnly,
        }
    }
}

pub fn is_number(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Try to interpret `reference` as a 1-based list index first;  
/// if that fails, fall back to a (possibly-shortened) task-ID.
pub fn resolve_task(
    manager: &TaskManager,
    reference: &str,
    show_all: bool,
) -> Result<Option<Task>, TaskError> {
    if is_number(reference) {
        let idx: usize = reference.parse().unwrap_or(0);
        let filter = if show_all {
            StatusFilter::All
        } else {
            StatusFilter::PendingOnly
        };
        if let Some(t) = manager
            .list_tasks(filter)?
            .into_iter()
            .nth(idx.saturating_sub(1))
        {
            return Ok(Some(t));
        }
    }
    manager.find_task_by_id(reference)
}

fn get_tarea_dir() -> Result<PathBuf, TaskError> {
    let home = env::var("HOME").map_err(|_| {
        TaskError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "HOME environment variable not found",
        ))
    })?;

    let tarea_dir = PathBuf::from(home).join(".tarea");
    if !tarea_dir.exists() {
        fs::create_dir_all(&tarea_dir)?;
    }
    Ok(tarea_dir)
}

fn last_list_all_path() -> Result<PathBuf, TaskError> {
    Ok(get_tarea_dir()?.join("last_list_all"))
}

pub fn save_last_list_all(all: bool) -> Result<(), TaskError> {
    let path = last_list_all_path()?;
    if all {
        fs::write(path, b"1")?;
    } else {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

pub fn was_last_list_all() -> bool {
    last_list_all_path().ok().is_some_and(|p| p.exists())
}

pub fn delete_database() -> Result<(), TaskError> {
    use std::io::Write;

    print!("Are you sure you want to delete the database? This action cannot be undone. (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if matches!(input.as_str(), "y" | "yes") {
        let db_path = crate::database::get_db_path()?;
        match fs::remove_file(&db_path) {
            Ok(_) => println!("{}", "Database deleted successfully".bright_green()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                println!("{}", "Database file not found".bright_yellow())
            }
            Err(e) => return Err(TaskError::Io(e)),
        }
    } else {
        println!("{}", "Database deletion cancelled".bright_yellow());
    }
    Ok(())
}
