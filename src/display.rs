use crate::types::{Status, Task};
use chrono::{DateTime, Duration, NaiveDateTime, Timelike, Utc};
use colored::*;
use terminal_size::{Width, terminal_size};
use textwrap::wrap;

const WRAP_COLUMN: usize = 80;
const MIN_DESCRIPTION_INDENT: usize = 3;
const DOT_STATUS_CHARACTER: char = '●';
const SHORT_ID_LENGTH: usize = 8;
const SIGN_LATE: char = '!';
const SIGN_SOON: char = '*';
const SIGN_DUE: char = '-';

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusDisplay {
    Dot,
    Word,
}

pub fn format_status_char(status: &Status, display: StatusDisplay) -> ColoredString {
    let dot = DOT_STATUS_CHARACTER.to_string();
    match display {
        StatusDisplay::Dot => match status {
            Status::Done => dot.bright_green(),
            Status::Pending => dot.bright_yellow(),
            Status::Standby => dot.bright_blue(),
        },
        StatusDisplay::Word => match status {
            Status::Done => "[d]".bright_green(),
            Status::Pending => "[p]".bright_yellow(),
            Status::Standby => "[s]".bright_blue(),
        },
    }
}

pub fn format_task_line_with_number(
    number: usize,
    number_width: usize,
    task: &Task,
    name_width: usize,
    time_width: usize,
    indent_len: usize,
    time_col_start: usize,
    show_description: bool,
    status_display: StatusDisplay,
) {
    print!("{:>width$}. ", number, width = number_width);
    format_task_line(
        task,
        name_width,
        time_width,
        indent_len,
        time_col_start,
        show_description,
        status_display,
    );
}

pub fn format_task_line(
    task: &Task,
    name_width: usize,
    time_width: usize,
    indent_len: usize,
    time_col_start: usize,
    show_description: bool,
    status_display: StatusDisplay,
) {
    let status_char = format_status_char(&task.status, status_display);
    let is_done = task.status == Status::Done;

    let short_id = &task.id[..SHORT_ID_LENGTH.min(task.id.len())];
    let display_name = truncate_with_dots(&task.name, name_width);

    let created_dt = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str(&task.date, "%Y-%m-%d %H:%M:%S").unwrap(),
        Utc,
    );
    let created_str = pretty_time(created_dt);
    let mut date_display = format!("{:>width$}", created_str, width = time_width)
        .dimmed()
        .to_string();

    if !is_done {
        if let Some(ref due_date) = task.due_date {
            let due_str = pretty_time(*due_date);
            let overdue = *due_date < Utc::now();
            let icon = if overdue {
                SIGN_LATE
            } else if is_due_soon(due_date) {
                SIGN_SOON
            } else {
                SIGN_DUE
            };
            let due_display = if overdue {
                format!("{} {} (late)", icon, due_str).bright_red()
            } else if is_due_soon(due_date) {
                format!("{} {}", icon, due_str).bright_yellow()
            } else {
                format!("{} {}", icon, due_str).dimmed()
            };
            date_display = format!("{} {}", date_display, due_display);
        }
    }

    println!(
        "{} {} {:<width$} {}",
        format!("{:>3}", short_id).bright_black(),
        status_char,
        display_name.bright_white(),
        date_display,
        width = name_width
    );

    if show_description && !task.description.is_empty() {
        print_task_description_formatted(task, indent_len, time_col_start);
    }
}

fn print_task_description_formatted(task: &Task, indent_len: usize, time_col_start: usize) {
    // blank line above description
    println!();

    let indent = " ".repeat(indent_len.max(MIN_DESCRIPTION_INDENT));

    // preferred wrap column is 80 if the terminal is wide enough,
    // otherwise we stop *just* before the timestamp column so the two
    // never collide.
    let wrap_limit = if term_width() >= WRAP_COLUMN {
        WRAP_COLUMN
    } else {
        // leave one spare column so we never touch the date
        time_col_start.saturating_sub(1)
    };

    let wrap_width = wrap_limit.saturating_sub(indent_len);

    for line in wrap(&task.description, wrap_width) {
        println!("{}{}", indent, line.dimmed());
    }

    // blank line below description
    println!();
}

pub fn print_task_details(task: &Task, minimal_mode: bool) {
    let pad = 8;
    print_task_id(task, pad);
    print_task_name(task, pad);
    print_task_description(task, pad);
    if !minimal_mode {
        print_task_created(task, pad);
    }
    print_task_due_date(task, pad);
    if !minimal_mode {
        print_task_status(task, pad, StatusDisplay::Dot);
    }
}

fn print_task_id(task: &Task, pad: usize) {
    println!("{:<pad$} {}", "id".dimmed(), task.id.bright_white());
}

fn print_task_name(task: &Task, pad: usize) {
    println!("{:<pad$} {}", "name".dimmed(), task.name.bold());
}

fn print_task_description(task: &Task, pad: usize) {
    if task.description.is_empty() {
        return;
    }

    let indent_len = (pad + 1).max(MIN_DESCRIPTION_INDENT);
    let indent = " ".repeat(indent_len);

    let term_w = term_width();
    let wrap_limit = if term_w >= WRAP_COLUMN {
        WRAP_COLUMN
    } else {
        term_w.saturating_sub(1)
    };
    let wrap_width = wrap_limit.saturating_sub(indent_len);

    let wrapped = textwrap::wrap(&task.description, wrap_width);

    if let Some((first, rest)) = wrapped.split_first() {
        println!("{:<pad$} {}", "details".dimmed(), first.dimmed(), pad = pad);

        for line in rest {
            println!("{}{}", indent, line.dimmed());
        }

        if !rest.is_empty() {
            println!();
        }
    }
}

fn print_task_created(task: &Task, pad: usize) {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::parse_from_str(&task.date, "%Y-%m-%d %H:%M:%S").unwrap(),
        Utc,
    );
    println!(
        "{:<pad$} {}",
        "created".dimmed(),
        pretty_time(dt),
        pad = pad
    );
}

fn print_task_due_date(task: &Task, pad: usize) {
    if let Some(ref due_date) = task.due_date {
        let due_str = pretty_time(*due_date);
        let icon = if *due_date < Utc::now() {
            SIGN_LATE
        } else if is_due_soon(due_date) {
            SIGN_SOON
        } else {
            SIGN_DUE
        };
        let overdue = *due_date < Utc::now();
        let due_display = if overdue {
            format!("{} {} (late)", icon, due_str).bright_red()
        } else if is_due_soon(due_date) {
            format!("{} {}", icon, due_str).bright_yellow()
        } else {
            format!("{} {}", icon, due_str).dimmed()
        };

        println!("{:<pad$} {}", "due".dimmed(), due_display);
    }
}

fn print_task_status(task: &Task, pad: usize, display: StatusDisplay) {
    let out = format_status_char(&task.status, display);
    println!("{:<pad$} {}", "status".dimmed(), out, pad = pad);
}

pub fn pretty_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let secs = (dt - now).num_seconds();
    let future = secs >= 0;
    let abs_secs = secs.abs();

    // TODO: Better solution would be to track the original format from database
    // and pass it here, so we can distinguish between user-specified midnight
    // (e.g., "2025-08-12 00:00") and date-only input (e.g., "2025-08-12").
    // For now, we use heuristics:
    // - Exact midnight local time = date-only input (like "2025-08-12")  
    // - 23:59:59 local time = "today"/"tomorrow" input (should use relative time)
    let local_dt = dt.with_timezone(&chrono::Local);
    let is_date_only = local_dt.time().hour() == 0 && local_dt.time().minute() == 0 && local_dt.time().second() == 0;

    // Use relative time for "today"/"tomorrow" (23:59:59) and other time-specific tasks
    // But never for date-only tasks (00:00:00)
    if !is_date_only && abs_secs < 86_400 {
        let mins = (abs_secs + 59) / 60;
        let hours = mins / 60;
        let minutes = mins % 60;

        let mut parts = Vec::new();
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if parts.is_empty() {
            parts.push("0m".into());
        }

        let phrase = parts.join(" ");
        return if future {
            format!("in {}", phrase)
        } else {
            format!("{} ago", phrase)
        };
    }

    // For date-only tasks, use local date for comparison to show correct date
    // For time-specific tasks, use UTC date as before
    let (d, nd) = if is_date_only {
        let local_dt = dt.with_timezone(&chrono::Local);
        let local_now = now.with_timezone(&chrono::Local);
        (local_dt.date_naive(), local_now.date_naive())
    } else {
        (dt.date_naive(), now.date_naive())
    };
    let diff_days = (d - nd).num_days();

    match diff_days {
        0 if is_date_only => "today".to_string(),
        0 => format!("today at {}", dt.with_timezone(&chrono::Local).format("%H:%M")),
        -1 if is_date_only => "yesterday".to_string(),
        -1 => format!("yesterday at {}", dt.with_timezone(&chrono::Local).format("%H:%M")),
        1 if is_date_only => "tomorrow".to_string(),
        1 => format!("tomorrow at {}", dt.with_timezone(&chrono::Local).format("%H:%M")),
        -6..=6 if is_date_only => dt.with_timezone(&chrono::Local).format("%A").to_string(),
        -6..=6 => dt.with_timezone(&chrono::Local).format("%A at %H:%M").to_string(),
        _ if is_date_only => dt.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string(),
        _ => dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(),
    }
}

fn truncate_with_dots(s: &str, limit: usize) -> String {
    if s.len() <= limit {
        return s.to_string();
    }

    let truncated: String = s.chars().take(limit - 3).collect();
    format!("{}...", truncated)
}

fn is_due_soon(due_date: &DateTime<Utc>) -> bool {
    let now = Utc::now();
    let diff = *due_date - now;
    if diff.num_seconds() < 0 {
        return false;
    }

    if diff <= Duration::minutes(20) {
        return true; // minute‑level tasks
    }

    if diff <= Duration::hours(24) {
        return true; // "today" or specific‑date tasks (day‑before window)
    }

    if diff <= Duration::days(7) {
        return diff <= Duration::days(1); // week‑range tasks
    }

    diff <= Duration::days(3) // longer‑range tasks
}

fn term_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone, Utc};

    #[test]
    fn test_pretty_time_date_only_shows_no_time() {
        // Create a date-only task (midnight local time converted to UTC)
        let local_midnight = Local.with_ymd_and_hms(2025, 8, 15, 0, 0, 0).unwrap();
        let utc_dt = local_midnight.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should show the original date entered by user (2025-08-15) without time
        assert!(result.contains("2025-08-15") && !result.contains(":"));
    }

    #[test]  
    fn test_pretty_time_with_time_shows_local_time() {
        // Create a datetime task (15:30 local time converted to UTC)
        let local_dt = Local.with_ymd_and_hms(2025, 8, 15, 15, 30, 0).unwrap();
        let utc_dt = local_dt.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should show the original local time (15:30), not UTC time
        assert!(result.contains("15:30"));
    }

    #[test]
    fn test_pretty_time_date_only_within_week_shows_weekday() {
        // Test a date-only task that falls within the current week (but not today/tomorrow)
        let now = Local::now();
        let in_three_days = now.date_naive() + chrono::Duration::days(3);
        let future_midnight = Local.from_local_datetime(&in_three_days.and_hms_opt(0, 0, 0).unwrap()).single().unwrap();
        let utc_dt = future_midnight.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should show weekday name without time for date-only tasks within week range
        let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
        let shows_weekday = days.iter().any(|day| result.contains(day));
        let has_no_time = !result.contains(":");
        
        // Should either show weekday or special case like "tomorrow" - both are acceptable for date-only
        assert!(has_no_time && (shows_weekday || result == "tomorrow" || result == "today"));
    }

    #[test]
    fn test_pretty_time_end_of_day_shows_relative_time() {
        // Test "today"/"tomorrow" tasks (saved as 23:59:59)
        let now = Local::now();
        let today_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap()
            .and_local_timezone(Local).unwrap();
        let utc_dt = today_end.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should show relative time format for today/tomorrow tasks
        assert!(result.contains("in ") || result.contains(" ago"));
    }

    #[test]
    fn test_pretty_time_timezone_consistency() {
        // Test that input time matches output time (timezone consistency)
        let local_time = Local.with_ymd_and_hms(2025, 8, 15, 14, 30, 0).unwrap();
        let utc_time = local_time.with_timezone(&Utc);
        
        let result = pretty_time(utc_time);
        
        // The displayed time should match the original local time
        assert!(result.contains("14:30"));
        assert!(!result.contains("12:30")); // Should not show UTC time
    }

    #[test]
    fn test_pretty_time_date_only_far_future() {
        // Test date-only task far in the future
        let future_date = Local.with_ymd_and_hms(2026, 12, 25, 0, 0, 0).unwrap();
        let utc_dt = future_date.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should show the original date entered by user without time
        assert!(result.contains("2026-12-25"));
        assert!(!result.contains(":"));
    }

    #[test]
    fn test_pretty_time_preserves_user_midnight() {
        // Test that user-specified midnight (00:00) is treated as date-only
        let user_midnight = Local.with_ymd_and_hms(2025, 8, 15, 0, 0, 0).unwrap();
        let utc_dt = user_midnight.with_timezone(&Utc);
        
        let result = pretty_time(utc_dt);
        
        // Should be treated as date-only (this is the limitation mentioned in TODO)
        // but it's the current expected behavior
        assert!(!result.contains(":") || result == "today" || result == "tomorrow");
    }
}
