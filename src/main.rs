use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::{Arg, Command};
use clap_complete::generate;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};
use colored::*;
use rusqlite::{Connection, Result as SqlResult};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fmt, fs};
use terminal_size::{Width, terminal_size};
use textwrap::wrap;
use uuid::Uuid;

const DESCRIPTION_INDENTATION_LENGHT: usize = 12;
const DOT_STATUS_CHARACTER: char = '●';
const MAX_TASK_NAME_LENGTH: usize = 120;
const SHORT_ID_LENGTH: usize = 8;
const SIGN_LATE: char = '!';
const SIGN_SOON: char = '*';
const SIGN_DUE: char = '-';
const DYNAMIC_COMPLETE_BASH: &str = r#"
__tarea_ids() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    COMPREPLY=( $(compgen -W "$(tarea --ids --short 2>/dev/null)" -- "$cur") )
}

# after these flags we want ID completion:
for opt in --show --edit --done --pending --standby; do
    complete -o default -F __tarea_ids tarea $opt
done
"#;
// const DYNAMIC_COMPLETE_ZSH: &str = r#"
// # Tiny helper that prints all short IDs
// _tarea_ids() { tarea --ids --short 2>/dev/null }
//
// # Keep clap’s original function and wrap it
// if ! typeset -f _tarea_orig >/dev/null; then
//   functions[_tarea_orig]=$functions[_tarea]
// fi
//
// _tarea() {
//   # First let the auto-generated function do its job
//   _tarea_orig "$@" && return
//
//   # Then add our dynamic IDs for the flags that expect one
//   _arguments -C \
//     '--show[show task]:task ID:_tarea_ids' \
//     '--edit[edit task]:task ID:_tarea_ids' \
//     '--done[mark done]:task ID:_tarea_ids' \
//     '--pending[mark pending]:task ID:_tarea_ids' \
//     '--standby[mark standby]:task ID:_tarea_ids' && return
// }
// "#;
const DYNAMIC_COMPLETE_FISH: &str = r#"
function __tarea_ids
    tarea --ids --short ^/dev/null
end

# Attach dynamic ID completion to each flag that takes a TASK_ID
for opt in show edit done pending standby
    complete -c tarea -n "__fish_seen_argument -l $opt" -a "(__tarea_ids)"
end
"#;

#[derive(Debug)]
enum TaskError {
    Database(rusqlite::Error),
    InvalidDate(String),
    InvalidId(String),
    InvalidInput(String),
    Io(io::Error),
}

impl From<rusqlite::Error> for TaskError {
    fn from(err: rusqlite::Error) -> Self {
        TaskError::Database(err)
    }
}

impl From<io::Error> for TaskError {
    fn from(err: io::Error) -> Self {
        TaskError::Io(err)
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Database(e) => write!(f, "{} {}", "Database error:".bright_red(), e),
            TaskError::Io(e) => write!(f, "{} {}", "IO error:".bright_red(), e),
            TaskError::InvalidId(e) => write!(f, "{} {}", "Invalid ID:".bright_yellow(), e),
            TaskError::InvalidDate(e) => write!(f, "{} {}", "Invalid date:".bright_yellow(), e),
            TaskError::InvalidInput(e) => write!(f, "{} {}", "Invalid input:".bright_yellow(), e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Pending,
    Done,
    Standby,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::Done => write!(f, "done"),
            Status::Standby => write!(f, "standby"),
        }
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Status::Pending),
            "done" => Ok(Status::Done),
            "standby" => Ok(Status::Standby),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

#[derive(Debug)]
struct Task {
    id: String,
    date: String,
    name: String,
    description: String,
    status: Status,
    due_date: Option<DateTime<Utc>>,
}

impl Task {
    fn new(
        name: String,
        description: Option<String>,
        due_date: Option<DateTime<Utc>>,
    ) -> Result<Self, TaskError> {
        validate_task_name(&name)?;

        Ok(Task {
            id: Uuid::new_v4().to_string(),
            date: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            name,
            description: description.unwrap_or_default(),
            status: Status::Pending,
            due_date,
        })
    }
}

#[derive(Debug)]
enum TaskCommand {
    Add {
        name: String,
        description: Option<String>,
        due_date: Option<DateTime<Utc>>,
    },
    Completions {
        shell: String,
    },
    DeleteDatabase,
    Edit {
        id_or_index: String,
        show_all: bool,
        field: EditField,
    },
    List {
        status: Option<Status>,
        show_all: bool,
        show_descriptions: bool,
    },
    ListNames {
        show_all: bool,
    },
    Show {
        id: String,
        show_all: bool,
    },
    ShowName {
        id_or_index: String,
        show_all: bool,
    },
    UpdateStatus {
        id: String,
        status: Status,
    },
    Ids {
        short_only: bool,
    },
}

#[derive(Debug)]
enum EditField {
    Name(String),
    Description(String),
    DueDate(DateTime<Utc>),
}

struct TaskManager {
    conn: Connection,
}

impl TaskManager {
    fn new() -> Result<Self, TaskError> {
        let conn = init_db()?;
        Ok(TaskManager { conn })
    }

    fn add_task(&self, task: Task) -> Result<(), TaskError> {
        let due_date_str = task
            .due_date
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        self.conn.execute(
            "INSERT INTO tasks (id, date, name, description, status, due_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &task.id,
                &task.date,
                &task.name,
                &task.description,
                &task.status.to_string(),
                &due_date_str,
            ],
        )?;
        Ok(())
    }

    fn list_tasks(&self, status: Option<Status>, show_all: bool) -> Result<Vec<Task>, TaskError> {
        let (query, params) = build_task_query(status.as_ref(), show_all);
        let mut stmt = self.conn.prepare(query)?;

        let mut tasks = Vec::new();
        let row_mapper = |row: &rusqlite::Row| self.row_to_task(row);

        let task_iter = if let Some(status_param) = params {
            stmt.query_map([status_param], row_mapper)?
        } else {
            stmt.query_map([], row_mapper)?
        };

        for task_result in task_iter {
            tasks.push(task_result?);
        }

        Ok(tasks)
    }

    fn find_task_by_id(&self, short_id: &str) -> Result<Option<Task>, TaskError> {
        let matching_ids = self.find_matching_ids(short_id)?;

        match matching_ids.len() {
            0 => Ok(None),
            1 => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, date, name, description, status, due_date FROM tasks WHERE id = ?1",
                )?;
                let mut rows = stmt.query_map([&matching_ids[0]], |row| self.row_to_task(row))?;

                if let Some(task_result) = rows.next() {
                    Ok(Some(task_result?))
                } else {
                    Ok(None)
                }
            }
            _ => Err(TaskError::InvalidId(format!(
                "Ambiguous ID '{}', matches: {}",
                short_id,
                matching_ids
                    .iter()
                    .map(|id| &id[..SHORT_ID_LENGTH])
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    fn update_task_status(&self, short_id: &str, new_status: Status) -> Result<bool, TaskError> {
        let matching_ids = self.find_matching_ids(short_id)?;

        match matching_ids.len() {
            0 => Ok(false),
            1 => {
                let updated = self.conn.execute(
                    "UPDATE tasks SET status = ?1 WHERE id = ?2",
                    [&new_status.to_string(), &matching_ids[0]],
                )?;
                Ok(updated > 0)
            }
            _ => Err(TaskError::InvalidId(format!(
                "Ambiguous ID '{}', matches: {}",
                short_id,
                matching_ids
                    .iter()
                    .map(|id| &id[..SHORT_ID_LENGTH])
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    fn find_matching_ids(&self, short_id: &str) -> Result<Vec<String>, TaskError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks WHERE id LIKE ?1 || '%'")?;
        let mut ids = Vec::new();

        let rows = stmt.query_map([short_id], |row| row.get::<_, String>(0))?;

        for id_result in rows {
            ids.push(id_result?);
        }

        Ok(ids)
    }

    fn row_to_task(&self, row: &rusqlite::Row) -> SqlResult<Task> {
        let status_str: String = row.get(4)?;
        let status = Status::from_str(&status_str).unwrap_or(Status::Pending);
        let due_date_str: String = row.get(5)?;

        let due_date = if due_date_str.is_empty() {
            None
        } else {
            NaiveDateTime::parse_from_str(&due_date_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.and_utc())
        };

        Ok(Task {
            id: row.get(0)?,
            date: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            status,
            due_date,
        })
    }
    fn update_name(&self, id: &str, name: &str) -> Result<bool, TaskError> {
        validate_task_name(name)?;
        Ok(self
            .conn
            .execute("UPDATE tasks SET name = ?1 WHERE id = ?2", [name, id])?
            > 0)
    }

    fn update_description(&self, id: &str, desc: &str) -> Result<bool, TaskError> {
        Ok(self.conn.execute(
            "UPDATE tasks SET description = ?1 WHERE id = ?2",
            [desc, id],
        )? > 0)
    }

    fn update_due(&self, id: &str, due: Option<DateTime<Utc>>) -> Result<bool, TaskError> {
        let s = due
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        Ok(self
            .conn
            .execute("UPDATE tasks SET due_date = ?1 WHERE id = ?2", [&s, id])?
            > 0)
    }
}

fn validate_task_name(name: &str) -> Result<(), TaskError> {
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

fn term_width() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
}

fn parse_due_date(input: &str) -> Result<DateTime<Utc>, TaskError> {
    let trimmed = input.trim().to_lowercase();

    match trimmed.as_str() {
        "today" => {
            let date = Utc::now().date_naive();
            return Ok(date.and_hms_opt(23, 59, 59).unwrap().and_utc());
        }
        "tomorrow" => {
            let date = (Utc::now() + Duration::days(1)).date_naive();
            return Ok(date.and_hms_opt(23, 59, 59).unwrap().and_utc());
        }
        _ => {}
    }

    if let Some(h) = trimmed.strip_suffix('h') {
        if let Ok(n) = h.parse::<i64>() {
            return Ok(Utc::now() + Duration::hours(n));
        }
    }

    if let Some(m) = trimmed.strip_suffix('m') {
        if let Ok(n) = m.parse::<i64>() {
            return Ok(Utc::now() + Duration::minutes(n));
        }
    }

    let formats = ["%Y-%m-%d", "%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"];
    for fmt in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(&trimmed, fmt) {
            return Ok(dt.and_utc());
        }
    }

    Err(TaskError::InvalidDate(format!(
        "Unable to parse date '{}'. Use natural language like 'today', '2h' or an absolute date 'YYYY-MM-DD [HH:MM[:SS]]'",
        input
    )))
}

fn last_list_all_path() -> Result<PathBuf, TaskError> {
    Ok(get_tarea_dir()?.join("last_list_all"))
}

fn save_last_list_all(all: bool) -> Result<(), TaskError> {
    let path = last_list_all_path()?;
    if all {
        fs::write(path, b"1")?;
    } else {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn was_last_list_all() -> bool {
    last_list_all_path().ok().is_some_and(|p| p.exists())
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Try to interpret `reference` as a 1-based list index first;  
/// if that fails, fall back to a (possibly-shortened) task-ID.
fn resolve_task(
    manager: &TaskManager,
    reference: &str,
    show_all: bool,
) -> Result<Option<Task>, TaskError> {
    if is_number(reference) {
        let idx: usize = reference.parse().unwrap_or(0);
        if let Some(t) = manager
            .list_tasks(None, show_all)?
            .into_iter()
            .nth(idx.saturating_sub(1))
        {
            return Ok(Some(t));
        }
        // If there is no task with that index we fall through and
        // treat the string as an ID prefix (numeric UUIDs such as “97944653”).
    }
    manager.find_task_by_id(reference)
}

fn format_task_line_with_number(
    number: usize,
    number_width: usize,
    task: &Task,
    name_width: usize,
    time_width: usize,
    show_description: bool,
    status_display: StatusDisplay,
) {
    print!("{:>width$}. ", number, width = number_width);
    format_task_line(
        task,
        name_width,
        time_width,
        show_description,
        status_display,
    );
}

fn build_task_query(
    status_filter: Option<&Status>,
    show_all: bool,
) -> (&'static str, Option<String>) {
    match (show_all, status_filter) {
        (true, _) => (
            "SELECT id, date, name, description, status, due_date FROM tasks ORDER BY date DESC",
            None,
        ),
        (false, Some(status)) => (
            "SELECT id, date, name, description, status, due_date FROM tasks WHERE status = ?1 ORDER BY date DESC",
            Some(status.to_string()),
        ),
        (false, None) => (
            "SELECT id, date, name, description, status, due_date FROM tasks WHERE status = 'pending' ORDER BY date DESC",
            None,
        ),
    }
}

fn cli() -> Command {
    Command::new("tarea")
        .about("A simple task manager")
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .help("Show all tasks regardless of status")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("completions")
                .long("completions")
                .help("Print completion script for <SHELL> to stdout")
                .value_parser(["bash", "zsh", "fish", "powershell", "elvish"])
                .value_name("SHELL"),
        )
        .arg(
            Arg::new("delete-database")
                .long("delete-database")
                .help("Delete the task database")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("description")
                .short('d')
                .long("desc")
                .help("Show task descriptions in list, or add description if text provided")
                .num_args(0..)
                .value_name("DESCRIPTION"),
        )
        .arg(
            Arg::new("done")
                .long("done")
                .help("Mark task as done (if TASK_ID given) or list done tasks")
                .num_args(0..=1)
                .value_name("TASK_ID"),
        )
        .arg(
            Arg::new("due-date")
                .long("due")
                .help("Set due date (today, tomorrow, 2h, 60m or YYYY-MM-DD [HH:MM[:SS]])")
                .num_args(1..)
                .value_name("DATE"),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .help("Print only task names (optionally a single task by INDEX/ID)")
                .num_args(0..=1)
                .value_name("TASK"),
        )
        .arg(
            Arg::new("edit")
                .short('e')
                .long("edit")
                .num_args(1)
                .help("Edit task name, description, or due date")
                .value_name("EDIT"),
        )
        .arg(
            Arg::new("pending")
                .long("pending")
                .help("Mark task as pending (if TASK_ID given) or list pending tasks")
                .num_args(0..=1)
                .value_name("TASK_ID"),
        )
        .arg(
            Arg::new("show")
                .long("show")
                .help("Show specific task by ID")
                .value_name("TASK_ID"),
        )
        .arg(
            Arg::new("standby")
                .long("standby")
                .help("Mark task as standby (if TASK_ID given) or list standby tasks")
                .num_args(0..=1)
                .value_name("TASK_ID"),
        )
        .arg(Arg::new("task").help("Task name to add").num_args(0..))
        .arg(
            Arg::new("ids")
                .short('i')
                .long("ids")
                .help("Print all task IDs (add --short for 8-char prefixes)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("short")
                .long("short")
                .help("Show shortened output when the chosen action supports it")
                .global(true)
                .action(clap::ArgAction::SetTrue),
        )
}

fn status_flag(matches: &clap::ArgMatches) -> Option<(Status, Option<String>)> {
    [
        ("done", Status::Done),
        ("pending", Status::Pending),
        ("standby", Status::Standby),
    ]
    .iter()
    .find_map(|(flag, st)| {
        matches
            .contains_id(flag)
            .then(|| (st.clone(), matches.get_one::<String>(flag).cloned()))
    })
}

fn parse_command() -> TaskCommand {
    let matches = cli().get_matches();

    if matches.get_flag("delete-database") {
        return TaskCommand::DeleteDatabase;
    }

    if matches.get_flag("ids") {
        return TaskCommand::Ids {
            short_only: matches.get_flag("short"),
        };
    }

    if let Some((status, id_opt)) = status_flag(&matches) {
        return match id_opt {
            Some(id) => TaskCommand::UpdateStatus { id, status },
            None => TaskCommand::List {
                status: Some(status),
                show_all: matches.get_flag("all"),
                show_descriptions: matches.contains_id("description"),
            },
        };
    }

    if let Some(id_val) = matches.get_one::<String>("edit") {
        let show_all = matches.get_flag("all");

        // --due  (date parsing reused)
        if let Some(due_vals) = matches.get_many::<String>("due-date") {
            let raw = due_vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
            let new_due = match parse_due_date(&raw) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };
            return TaskCommand::Edit {
                id_or_index: id_val.clone(),
                show_all,
                field: EditField::DueDate(new_due),
            };
        }

        // -d / --desc
        if let Some(desc_vals) = matches.get_many::<String>("description") {
            let desc = desc_vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
            if desc.is_empty() {
                eprintln!("No new description supplied");
                std::process::exit(1);
            }
            return TaskCommand::Edit {
                id_or_index: id_val.clone(),
                show_all,
                field: EditField::Description(desc),
            };
        }

        // name (implicit OR explicit --name)
        let new_name = if let Some(first) = matches.get_one::<String>("name") {
            let mut name_clone = first.clone();
            if let Some(rest) = matches.get_many::<String>("task") {
                let tail = rest.map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
                if !tail.is_empty() {
                    name_clone.push(' ');
                    name_clone.push_str(&tail);
                }
            }
            name_clone
        } else {
            matches
                .get_many::<String>("task")
                .map(|vals| vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    eprintln!("No new name supplied");
                    std::process::exit(1);
                })
        };

        return TaskCommand::Edit {
            id_or_index: id_val.clone(),
            show_all,
            field: EditField::Name(new_name),
        };
    }

    if matches.contains_id("name") {
        let id_opt = matches.get_one::<String>("name").cloned();
        return match id_opt {
            Some(id) => TaskCommand::ShowName {
                id_or_index: id,
                show_all: matches.get_flag("all"),
            },
            None => TaskCommand::ListNames {
                show_all: matches.get_flag("all"),
            },
        };
    }

    if let Some(task_id) = matches.get_one::<String>("show") {
        return TaskCommand::Show {
            id: task_id.clone(),
            show_all: matches.get_flag("all"),
        };
    }

    if let Some(shell) = matches.get_one::<String>("completions") {
        return TaskCommand::Completions {
            shell: shell.clone(),
        };
    }

    let task_name = matches
        .get_many::<String>("task")
        .map(|vals| vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty());

    if let Some(name) = task_name {
        let description = if let Some(desc_vals) = matches.get_many::<String>("description") {
            let desc_text = desc_vals.map(|s| s.as_str()).collect::<Vec<_>>();
            if desc_text.is_empty() {
                None
            } else {
                Some(desc_text.join(" "))
            }
        } else {
            None
        };

        let due_date = if let Some(date_vals) = matches.get_many::<String>("due-date") {
            let date_str = date_vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
            match parse_due_date(&date_str) {
                Ok(dt) => Some(dt),
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        } else {
            None
        };

        return TaskCommand::Add {
            name,
            description,
            due_date,
        };
    }

    let show_descriptions = if let Some(desc_vals) = matches.get_many::<String>("description") {
        desc_vals.collect::<Vec<_>>().is_empty()
    } else {
        matches.contains_id("description")
    };

    let show_all = matches.get_flag("all");

    TaskCommand::List {
        status: None,
        show_all,
        show_descriptions,
    }
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

fn get_db_path() -> Result<PathBuf, TaskError> {
    Ok(get_tarea_dir()?.join("tasks.db"))
}

fn init_db() -> Result<Connection, TaskError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            date TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'pending'
        )",
        [],
    )?;

    // Add due_date column if it doesn't exist
    conn.execute("ALTER TABLE tasks ADD COLUMN due_date TEXT", [])
        .or_else(|_| Ok::<usize, rusqlite::Error>(0))?;

    Ok(conn)
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
        return true; // “today” or specific‑date tasks (day‑before window)
    }

    if diff <= Duration::days(7) {
        return diff <= Duration::days(1); // week‑range tasks
    }

    diff <= Duration::days(3) // longer‑range tasks
}

fn format_task_line(
    task: &Task,
    name_width: usize,
    time_width: usize,
    show_description: bool,
    status_display: StatusDisplay,
) {
    let dot = DOT_STATUS_CHARACTER.to_string();
    let is_done = task.status == Status::Done;

    let status_char = match status_display {
        StatusDisplay::Dot => match task.status {
            Status::Done => dot.bright_green(),
            Status::Pending => dot.bright_yellow(),
            Status::Standby => dot.bright_blue(),
        },
        StatusDisplay::Word => match task.status {
            Status::Done => "[d]".bright_green(),
            Status::Pending => "[p]".bright_yellow(),
            Status::Standby => "[s]".bright_blue(),
        },
    };

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
                format!("{} {}", icon, due_str).truecolor(255, 165, 0)
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
        let indent = " ".repeat(DESCRIPTION_INDENTATION_LENGHT);
        let wrap_width = term_width().saturating_sub(DESCRIPTION_INDENTATION_LENGHT);
        for line in wrap(&task.description, wrap_width) {
            println!("{}{}", indent, line.dimmed());
        }
    }
}

fn print_task_id(task: &Task, pad: usize) {
    println!("{:<pad$} {}", "id".dimmed(), task.id.bright_white());
}

fn print_task_name(task: &Task, pad: usize) {
    println!("{:<pad$} {}", "name".dimmed(), task.name.bold());
}

fn print_task_description(task: &Task, pad: usize) {
    if !task.description.is_empty() {
        println!("{:<pad$} {}", "details".dimmed(), task.description);
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

enum StatusDisplay {
    Dot,
    Word,
}

fn print_task_status(task: &Task, pad: usize, display: StatusDisplay) {
    let dot = DOT_STATUS_CHARACTER.to_string();
    let out = match display {
        StatusDisplay::Dot => match task.status {
            Status::Done => dot.bright_green(),
            Status::Pending => dot.bright_yellow(),
            Status::Standby => dot.bright_blue(),
        },
        StatusDisplay::Word => match task.status {
            Status::Done => "done".bright_green(),
            Status::Pending => "pending".bright_yellow(),
            Status::Standby => "standby".bright_blue(),
        },
    };

    println!("{:<pad$} {}", "status".dimmed(), out, pad = pad);
}

fn execute_command(manager: &TaskManager, command: TaskCommand) -> Result<(), TaskError> {
    match command {
        TaskCommand::Add {
            name,
            description,
            due_date,
        } => {
            let task = Task::new(name.clone(), description, due_date)?;
            manager.add_task(task)?;
            println!("{} {}", "task saved:".bright_green(), name);
        }
        TaskCommand::Completions { shell } => {
            let mut cmd = cli();

            match shell.as_str() {
                "bash" => {
                    generate(Bash, &mut cmd, "tarea", &mut io::stdout());
                    print!("{DYNAMIC_COMPLETE_BASH}");
                }
                "zsh" => {
                    generate(Zsh, &mut cmd, "tarea", &mut io::stdout());
                    // print!("{DYNAMIC_COMPLETE_ZSH}"); // FIX zsh ids completions
                }
                "fish" => {
                    generate(Fish, &mut cmd, "tarea", &mut io::stdout());
                    // print!("{DYNAMIC_COMPLETE_FISH}"); // FIX fish ids completions
                }
                "powershell" => generate(PowerShell, &mut cmd, "tarea", &mut io::stdout()),
                "elvish" => generate(Elvish, &mut cmd, "tarea", &mut io::stdout()),
                _ => unreachable!(),
            };
        }
        TaskCommand::List {
            status,
            show_all,
            show_descriptions,
        } => {
            let tasks = manager.list_tasks(status.clone(), show_all)?;

            if tasks.is_empty() {
                let message = match (show_all, status) {
                    (true, _) => "no tasks found".to_string(),
                    (false, Some(s)) => format!("no {} tasks found", s),
                    (false, None) => "no pending tasks found".to_string(),
                };
                println!("{}", message.dimmed());
                return Ok(());
            }

            let number_width = tasks.len().to_string().len();

            let created_width = tasks
                .iter()
                .map(|t| {
                    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
                        NaiveDateTime::parse_from_str(&t.date, "%Y-%m-%d %H:%M:%S").unwrap(),
                        Utc,
                    );
                    pretty_time(dt).len()
                })
                .max()
                .unwrap_or(0);

            let max_due_extra = tasks
                .iter()
                .map(|t| {
                    if t.status != Status::Done {
                        t.due_date
                            // leave one spare column so we never hit the right edge
                            .map(|d| 3 + pretty_time(d).len() + 1) // " * " == 3 cells
                            .unwrap_or(0)
                    } else {
                        0
                    }
                })
                .max()
                .unwrap_or(0);

            let term = term_width();
            let base_cols = number_width /* list index       */
                + 2                      /* ". "             */
                + SHORT_ID_LENGTH        /* short id         */
                + 1                      /* space            */
                + 1                      /* status dot       */
                + 1                      /* space            */
                + 1                      /* space after name */
                ;

            let time_width = created_width;
            let cap = term
                .saturating_sub(base_cols + time_width + max_due_extra)
                .max(10);

            // smallest width that still fits the longest *displayed* name
            let name_width = tasks
                .iter()
                .map(|t| truncate_with_dots(&t.name, cap).len())
                .max()
                .unwrap_or(10)
                .max(10);

            for (idx, task) in tasks.iter().enumerate() {
                format_task_line_with_number(
                    idx + 1,
                    number_width,
                    task,
                    name_width,
                    time_width,
                    show_descriptions,
                    StatusDisplay::Dot,
                );
            }
            save_last_list_all(show_all)?;
        }
        TaskCommand::ListNames { show_all } => {
            let tasks = manager.list_tasks(None, show_all)?;
            if tasks.is_empty() {
                println!("{}", "no tasks found".dimmed());
            } else {
                for (idx, t) in tasks.iter().enumerate() {
                    println!("{:>3}. {}", idx + 1, t.name);
                }
            }
        }
        TaskCommand::Show { id, show_all } => {
            // use the same --all / default view the user last displayed
            let use_all = if show_all { true } else { was_last_list_all() };

            let task_opt = resolve_task(manager, &id, use_all)?;

            match task_opt {
                Some(task) => {
                    let pad = 8;
                    print_task_id(&task, pad);
                    print_task_name(&task, pad);
                    print_task_description(&task, pad);
                    print_task_created(&task, pad);
                    print_task_due_date(&task, pad);
                    print_task_status(&task, pad, StatusDisplay::Dot); // TODO: Handle the status display via config or params
                }
                None => println!("{}", format!("Task '{}' not found", id).dimmed()),
            }
        }
        TaskCommand::ShowName {
            id_or_index,
            show_all,
        } => {
            let use_all = if show_all { true } else { was_last_list_all() };

            let task_opt = resolve_task(manager, &id_or_index, use_all)?;

            match task_opt {
                Some(t) => println!("{}", t.name),
                None => println!(
                    "{}",
                    format!("Task '{}' not found", id_or_index).bright_red()
                ),
            }
        }
        TaskCommand::Edit {
            id_or_index,
            show_all,
            field,
        } => {
            let use_all = if show_all { true } else { was_last_list_all() };

            let full_id = match resolve_task(manager, &id_or_index, use_all)? {
                Some(t) => t.id,
                None => {
                    println!(
                        "{}",
                        format!("Task '{}' not found", id_or_index).bright_red()
                    );
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
        }

        TaskCommand::UpdateStatus { id, status } => {
            let target_id = match resolve_task(manager, &id, was_last_list_all())? {
                Some(t) => t.id,
                None => {
                    println!("{}", format!("Task '{}' not found", id).bright_red());
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
                false => println!("{}", format!("Task '{}' not found", id).bright_red()),
            }
        }

        TaskCommand::DeleteDatabase => {
            delete_database()?;
        }
        TaskCommand::Ids { short_only } => {
            for t in manager.list_tasks(None, true)? {
                let out = if short_only {
                    &t.id[..SHORT_ID_LENGTH]
                } else {
                    &t.id
                };
                println!("{out}");
            }
        }
    }
    Ok(())
}

fn pretty_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let secs = (dt - now).num_seconds();

    if secs.abs() < 86_400 {
        if secs >= 0 {
            let mins = (secs + 59) / 60;
            return if mins < 60 {
                format!("in {}m", mins)
            } else {
                format!("in {}h", (mins + 59) / 60)
            };
        } else {
            let mins = (-secs + 59) / 60;
            return if mins < 60 {
                format!("{}m ago", mins)
            } else {
                format!("{}h ago", (mins + 59) / 60)
            };
        }
    }

    let d = dt.date_naive();
    let nd = now.date_naive();
    let diff_days = (d - nd).num_days();

    match diff_days {
        0 => format!("today at {}", dt.format("%H:%M")),
        -1 => format!("yesterday at {}", dt.format("%H:%M")),
        1 => format!("tomorrow at {}", dt.format("%H:%M")),
        -6..=6 => dt.format("%A at %H:%M").to_string(),
        _ => dt.format("%Y-%m-%d %H:%M").to_string(),
    }
}

fn delete_database() -> Result<(), TaskError> {
    print!("Are you sure you want to delete the database? This action cannot be undone. (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if matches!(input.as_str(), "y" | "yes") {
        let db_path = get_db_path()?;
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

fn main() {
    let command = parse_command();

    let manager = match TaskManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to initialize task manager: {}", e);
            return;
        }
    };

    if let Err(e) = execute_command(&manager, command) {
        eprintln!("{}", e);
    }
}
