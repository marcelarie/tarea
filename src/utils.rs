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
            return Ok((Local::now() + duration).with_timezone(&Utc));
        }
    }

    if let Some(mins_str) = cleaned.strip_suffix('m') {
        if let Ok(m) = mins_str.parse::<i64>() {
            return Ok((Local::now() + Duration::minutes(m)).with_timezone(&Utc));
        }
    }

    // Try date-only format first (defaults to 00:00:00)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(&trimmed, "%Y-%m-%d") {
        let naive = date.and_hms_opt(0, 0, 0).unwrap();
        match Local.from_local_datetime(&naive) {
            chrono::LocalResult::Single(local_dt) => {
                return Ok(local_dt.with_timezone(&Utc));
            }
            chrono::LocalResult::Ambiguous(_earlier, later) => {
                return Ok(later.with_timezone(&Utc));
            }
            chrono::LocalResult::None => {
                return Err(TaskError::InvalidDate(format!(
                    "Invalid local time '{}' (likely during DST transition)",
                    input
                )));
            }
        }
    }

    // Try date-time formats
    for fmt in ["%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&trimmed, fmt) {
            match Local.from_local_datetime(&naive) {
                chrono::LocalResult::Single(local_dt) => {
                    return Ok(local_dt.with_timezone(&Utc));
                }
                chrono::LocalResult::Ambiguous(_earlier, later) => {
                    // During DST "fall back":
                    // prefer the later (standard time) interpretation
                    return Ok(later.with_timezone(&Utc));
                }
                chrono::LocalResult::None => {
                    return Err(TaskError::InvalidDate(format!(
                        "Invalid local time '{}' (likely during DST transition)",
                        input
                    )));
                }
            }
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

pub fn format_task_not_found_message(id: &str, context: Option<&str>) -> impl std::fmt::Display {
    let base_msg = format!("task '{}'", id);
    let full_msg = match context {
        Some(ctx) => format!("{} not found{}", base_msg, ctx),
        None => format!("{} not found", base_msg),
    };
    full_msg.bright_red()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_task_name_valid() {
        assert!(validate_task_name("Valid task name").is_ok());
        assert!(validate_task_name("  Trimmed  ").is_ok());
    }

    #[test]
    fn test_validate_task_name_empty() {
        assert!(validate_task_name("").is_err());
        assert!(validate_task_name("   ").is_err());
    }

    #[test]
    fn test_validate_task_name_too_long() {
        let more_than_120_chars = "a".repeat(MAX_TASK_NAME_LENGTH + 1);
        assert!(validate_task_name(&more_than_120_chars).is_err());
        assert!(matches!(
            validate_task_name(&more_than_120_chars),
            Err(TaskError::InvalidInput(msg)) if msg.contains("Task name too long")
        ));
    }

    #[test]
    fn test_parse_due_date_today() {
        let today = Local::now().date_naive();
        let expected = today
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(parse_due_date("today").unwrap(), expected);
    }

    #[test]
    fn test_parse_due_date_tomorrow() {
        let tomorrow = (Local::now() + Duration::days(1)).date_naive();
        let expected = tomorrow
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(parse_due_date("tomorrow").unwrap(), expected);
    }

    #[test]
    fn test_parse_due_date_hours_minutes() {
        let now = Local::now();
        let result = parse_due_date("2h30m").unwrap();
        let expected = (now + Duration::hours(2) + Duration::minutes(30)).with_timezone(&Utc);

        // allow for up to 1 second timing differences
        let diff = (result - expected).abs();
        assert!(
            diff < Duration::seconds(1),
            "Difference too large: {:?}",
            diff
        );
    }

    #[test]
    fn test_parse_due_date_minutes() {
        let now = Local::now();
        let result = parse_due_date("30m").unwrap();
        let expected = (now + Duration::minutes(30)).with_timezone(&Utc);

        // allow for up to 1 second timing differences
        let diff = (result - expected).abs();
        assert!(
            diff < Duration::seconds(1),
            "Difference too large: {:?}",
            diff
        );
    }

    #[test]
    fn test_parse_due_date_just_hours() {
        let now = Local::now();
        let result = parse_due_date("2h").unwrap();
        let expected = (now + Duration::hours(2)).with_timezone(&Utc);

        let diff = (result - expected).abs();
        assert!(
            diff < Duration::seconds(1),
            "Difference too large: {:?}",
            diff
        );
    }

    #[test]
    fn test_parse_due_date_absolute_date_only() {
        let result = parse_due_date("2023-12-25").unwrap();
        let date = chrono::NaiveDate::parse_from_str("2023-12-25", "%Y-%m-%d").unwrap();
        let expected_naive = date.and_hms_opt(0, 0, 0).unwrap();
        let expected = Local
            .from_local_datetime(&expected_naive)
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_due_date_absolute_date_time() {
        let result = parse_due_date("2023-12-25 14:30").unwrap();
        let expected_naive =
            NaiveDateTime::parse_from_str("2023-12-25 14:30", "%Y-%m-%d %H:%M").unwrap();
        let expected = Local
            .from_local_datetime(&expected_naive)
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_due_date_absolute_date_time_seconds() {
        let result = parse_due_date("2023-12-25 14:30:45").unwrap();
        let expected_naive =
            NaiveDateTime::parse_from_str("2023-12-25 14:30:45", "%Y-%m-%d %H:%M:%S").unwrap();
        let expected = Local
            .from_local_datetime(&expected_naive)
            .single()
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_due_date_case_insensitive() {
        // Test that "TODAY" and "TOMORROW" work
        assert!(parse_due_date("TODAY").is_ok());
        assert!(parse_due_date("Tomorrow").is_ok());
        assert!(parse_due_date("TOMORROW").is_ok());
    }

    #[test]
    fn test_parse_due_date_whitespace_handling() {
        let now = Local::now();
        let result = parse_due_date("  2h 30m  ").unwrap();
        let expected = (now + Duration::hours(2) + Duration::minutes(30)).with_timezone(&Utc);

        let diff = (result - expected).abs();
        assert!(
            diff < Duration::seconds(1),
            "Difference too large: {:?}",
            diff
        );
    }

    #[test]
    fn test_parse_due_date_invalid_hour_minute_format() {
        // Hours followed by something other than 'm' should error
        assert!(parse_due_date("2h30").is_err());
        assert!(parse_due_date("2h30x").is_err());

        // Verify the error message
        let err = parse_due_date("2h30").unwrap_err();
        assert!(
            matches!(err, TaskError::InvalidDate(msg) if msg.contains("Expected minutes after hours"))
        );
    }

    #[test]
    fn test_parse_due_date_invalid_formats() {
        assert!(parse_due_date("invalid").is_err());
        assert!(parse_due_date("2023-13-01").is_err()); // Invalid month
        assert!(parse_due_date("2023-12-32").is_err()); // Invalid day
        assert!(parse_due_date("abc").is_err());
        assert!(parse_due_date("").is_err());
    }

    #[test]
    fn test_parse_due_date_negative_values() {
        // Negative hours/minutes should still parse but result in past dates
        let now = Local::now().with_timezone(&Utc);
        let result = parse_due_date("-1h").unwrap();
        assert!(result < now);
    }

    #[test]
    fn test_parse_due_date_zero_values() {
        let now = Local::now().with_timezone(&Utc);
        let result = parse_due_date("0h").unwrap();

        let diff = (result - now).abs();
        assert!(diff < Duration::seconds(1));
    }

    #[test]
    fn test_parse_due_date_large_values() {
        let now = Local::now();
        let result = parse_due_date("24h").unwrap();
        let expected = (now + Duration::hours(24)).with_timezone(&Utc);

        let diff = (result - expected).abs();
        assert!(diff < Duration::seconds(1));
    }
}
