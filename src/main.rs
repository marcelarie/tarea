use chrono::Utc;
use clap::{Arg, Command};
use colored::*;
use rusqlite::{Connection, Result};
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

const MAX_NAME_VIEW_LENGTH: usize = 70;
const SHORT_ID_LENGTH: usize = 8;
const DESCRIPTION_INDENTATION_LENGHT: usize = 12;

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
}

impl Task {
    fn new(name: String, description: Option<String>) -> Self {
        Task {
            id: Uuid::new_v4().to_string(),
            date: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            name,
            description: description.unwrap_or_default(),
            status: Status::Pending,
        }
    }
}

#[derive(Debug)]
struct Config {
    show_descriptions: bool,
    delete_database: bool,
    task_name: Option<String>,
    description: Option<String>,
}

fn parse_args() -> Config {
    let matches = Command::new("tarea")
        .about("A simple task manager")
        .arg(
            Arg::new("description")
                .short('d')
                .long("desc")
                .help("Show task descriptions in list, or add description if text provided")
                .num_args(0..=1)
                .value_name("DESCRIPTION"),
        )
        .arg(
            Arg::new("delete-database")
                .long("delete-database")
                .help("Delete the task database")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("task")
                .help("Task name to add")
                .trailing_var_arg(true)
                .num_args(0..),
        )
        .get_matches();

    let delete_database = matches.get_flag("delete-database");
    let task_name = matches
        .get_many::<String>("task")
        .map(|vals| vals.map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty());

    let (show_descriptions, description) =
        if let Some(desc_vals) = matches.get_many::<String>("description") {
            let desc_text = desc_vals.map(|s| s.as_str()).collect::<Vec<_>>();
            if desc_text.is_empty() {
                (true, None)
            } else {
                (false, Some(desc_text.join(" ")))
            }
        } else if matches.contains_id("description") {
            (true, None)
        } else {
            (false, None)
        };

    Config {
        show_descriptions,
        delete_database,
        task_name,
        description,
    }
}

fn get_tarea_dir() -> io::Result<PathBuf> {
    let home = env::var("HOME").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HOME environment variable not found",
        )
    })?;

    let tarea_dir = PathBuf::from(home).join(".tarea");
    if !tarea_dir.exists() {
        fs::create_dir_all(&tarea_dir)?;
    }
    Ok(tarea_dir)
}

fn get_db_path() -> io::Result<PathBuf> {
    Ok(get_tarea_dir()?.join("tasks.db"))
}

fn init_db() -> Result<Connection> {
    let db_path = get_db_path().map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some(format!("Cannot access tarea directory: {}", e)),
        )
    })?;

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
    Ok(conn)
}

fn save_task(conn: &Connection, task: &Task) -> Result<()> {
    conn.execute(
        "INSERT INTO tasks (id, date, name, description, status) VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            &task.id,
            &task.date,
            &task.name,
            &task.description,
            &task.status.to_string(),
        ],
    )?;
    Ok(())
}

fn truncate_with_dots(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }

    let truncated: String = s.chars().take(limit - 3).collect();
    format!("{}...", truncated)
}

fn format_task_line(task: &Task, name_width: usize, show_description: bool) {
    let status_char = match task.status {
        Status::Done => "[d]".bright_green(),
        Status::Pending => "[p]".bright_yellow(),
        Status::Standby => "[s]".bright_blue(),
    };

    let short_id = &task.id[..SHORT_ID_LENGTH.min(task.id.len())];
    let display_name = truncate_with_dots(&task.name, MAX_NAME_VIEW_LENGTH);

    println!(
        "{} {} {:<width$} {}",
        format!("{:>3}", short_id).bright_black(),
        status_char,
        display_name.bright_white(),
        task.date.dimmed(),
        width = name_width
    );

    if show_description && !task.description.is_empty() {
        println!(
            "{} {}",
            " ".repeat(DESCRIPTION_INDENTATION_LENGHT),
            task.description.dimmed()
        );
    }
}

fn list_tasks(conn: &Connection, show_descriptions: bool) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT id, date, name, description, status FROM tasks ORDER BY date DESC")?;

    let mut tasks = Vec::new();
    let task_iter = stmt.query_map([], |row| {
        let status_str: String = row.get(4)?;
        let status = Status::from_str(&status_str).unwrap_or(Status::Pending);

        Ok(Task {
            id: row.get(0)?,
            date: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            status,
        })
    })?;

    for task_result in task_iter {
        tasks.push(task_result?);
    }

    if tasks.is_empty() {
        println!("{}", "no tasks found".dimmed());
        return Ok(());
    }

    let name_width = tasks
        .iter()
        .map(|t| truncate_with_dots(&t.name, MAX_NAME_VIEW_LENGTH).len())
        .max()
        .unwrap_or(0);

    for task in &tasks {
        format_task_line(task, name_width, show_descriptions);
    }

    Ok(())
}

fn delete_database() -> io::Result<()> {
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
            Err(e) => return Err(e),
        }
    } else {
        println!("{}", "Database deletion cancelled".bright_yellow());
    }
    Ok(())
}

fn main() {
    let config = parse_args();

    if config.delete_database {
        if let Err(e) = delete_database() {
            eprintln!("Error: {}", e);
        }
    } else if let Some(task_name) = config.task_name {
        let conn = match init_db() {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Database error: {}", e);
                return;
            }
        };

        let task = Task::new(task_name.clone(), config.description);
        if let Err(e) = save_task(&conn, &task) {
            eprintln!("Error saving task: {}", e);
        } else {
            println!("{} {}", "task saved:".bright_green(), task_name);
        }
    } else {
        let conn = match init_db() {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Database error: {}", e);
                return;
            }
        };

        if let Err(e) = list_tasks(&conn, config.show_descriptions) {
            eprintln!("Error listing tasks: {}", e);
        }
    }
}
