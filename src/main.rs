use chrono::Utc;
use clap::Parser;
use colored::*;
use rusqlite::{Connection, Result};
use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

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

#[derive(Parser)]
#[command(name = "tarea")]
#[command(about = "A simple task manager")]
struct Args {
    /// The task name
    #[arg(trailing_var_arg = true)]
    #[arg(allow_hyphen_values = true)]
    task: Vec<String>,

    /// Task description
    #[arg(short, long)]
    description: Option<String>,

    /// Delete the database
    #[arg(long)]
    delete_database: bool,
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
    let tarea_dir = get_tarea_dir()?;
    Ok(tarea_dir.join("tasks.db"))
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
        "INSERT INTO tasks (id, date, name, description, status)
         VALUES (?1, ?2, ?3, ?4, ?5)",
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

fn pretty_print_tasks(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("{}", "no tasks found".dimmed());
        return;
    }

    let name_width = tasks.iter().map(|t| t.name.len()).max().unwrap_or(0);

    for task in tasks {
        let status_char = match task.status {
            Status::Done => "[d]".bright_green(),
            Status::Pending => "[p]".bright_yellow(),
            Status::Standby => "[s]".bright_blue(),
        };

        let short_id = &task.id[..8];
        println!(
            "{} {} {:<width$} {}",
            format!("{:>3}", short_id).bright_black(),
            status_char,
            task.name.bright_white(),
            task.date.dimmed(),
            width = name_width
        );

        if !task.description.is_empty() {
            println!("      {}", task.description.dimmed());
        }
    }
}

fn list_tasks(conn: &Connection) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT id, date, name, description, status FROM tasks ORDER BY date DESC")?;

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

    let mut tasks = Vec::new();
    for task_result in task_iter {
        let task = task_result?;
        tasks.push(task);
    }

    pretty_print_tasks(&tasks);
    Ok(())
}

fn delete_database() -> io::Result<()> {
    print!("Are you sure you want to delete the database? This action cannot be undone. (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if input == "y" || input == "yes" {
        let db_path = get_db_path()?;
        match fs::remove_file(&db_path) {
            Ok(_) => println!("{}", "Database deleted successfully".bright_green()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                println!("{}", "Database file not found".bright_yellow())
            }
            Err(e) => eprintln!("Error deleting database: {}", e),
        }
    } else {
        println!("{}", "Database deletion cancelled".bright_yellow());
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    if args.delete_database {
        match delete_database() {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
    } else if args.task.is_empty() {
        match init_db() {
            Ok(conn) => match list_tasks(&conn) {
                Ok(_) => {}
                Err(e) => eprintln!("Error listing tasks: {}", e),
            },
            Err(e) => eprintln!("Database error: {}", e),
        }
    } else {
        let task_name = args.task.join(" ");
        let task = Task::new(task_name.clone(), args.description);

        match init_db() {
            Ok(conn) => match save_task(&conn, &task) {
                Ok(_) => println!("{} {}", "task saved:".bright_green(), task_name),
                Err(e) => eprintln!("Error saving task: {}", e),
            },
            Err(e) => eprintln!("Database error: {}", e),
        }
    }
}
