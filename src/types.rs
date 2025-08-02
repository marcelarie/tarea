use chrono::{DateTime, Utc};
use colored::*;
use std::str::FromStr;
use std::{fmt, io};
use uuid::Uuid;

#[derive(Debug)]
pub enum TaskError {
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
pub enum Status {
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

pub enum StatusFilter {
    All,
    AnyOf(Vec<Status>),
    PendingOnly,
}

impl StatusFilter {
    pub fn to_sql(&self) -> (String, Vec<String>) {
        match self {
            StatusFilter::All => (String::new(), vec![]),

            StatusFilter::PendingOnly => (
                "WHERE status = ?1".into(),
                vec![Status::Pending.to_string()],
            ),

            StatusFilter::AnyOf(status) if status.is_empty() => {
                // fallback to pending-only if empty
                StatusFilter::PendingOnly.to_sql()
            }

            StatusFilter::AnyOf(status) => {
                let placeholders = std::iter::repeat("?")
                    .take(status.len())
                    .collect::<Vec<_>>()
                    .join(", ");
                let clause = format!("WHERE status IN ({})", placeholders);
                let params = status.iter().map(|status| status.to_string()).collect();
                (clause, params)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub date: String,
    pub name: String,
    pub description: String,
    pub status: Status,
    pub due_date: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(
        name: String,
        description: Option<String>,
        due_date: Option<DateTime<Utc>>,
    ) -> Result<Self, TaskError> {
        crate::utils::validate_task_name(&name)?;

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
pub enum TaskCommand {
    Add {
        name: String,
        description: Option<String>,
        due_date: Option<DateTime<Utc>>,
    },
    Completions {
        shell: String,
        dynamic_bash: String,
        dynamic_fish: String,
    },
    DeleteDatabase,
    Edit {
        id_or_index: String,
        field: EditField,
    },
    List {
        status: Option<Status>,
        show_all: bool,
        show_descriptions: bool,
    },
    ListNames {
        show_all: bool,
        status: Option<Status>,
    },
    Show {
        id: String,
    },
    ShowName {
        id_or_index: String,
        status: Option<Status>,
    },
    UpdateStatus {
        id: String,
        status: Status,
    },
    Ids {
        short_only: bool,
        filter: Vec<Status>,
    },
    Delete {
        id_or_index: String,
        status: Option<Status>,
    },
    EditWithEditor {
        id_or_index: String,
    },
}

#[derive(Debug)]
pub enum EditField {
    Name(String),
    Description(String),
    DueDate(DateTime<Utc>),
}
