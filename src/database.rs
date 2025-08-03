use crate::types::{Status, StatusFilter, Task, TaskError};
use crate::utils::validate_task_name;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{Connection, Result as SqlResult};
use std::io;
use std::path::PathBuf;
use std::{env, fs};

pub struct TaskManager {
    conn: Connection,
}

impl TaskManager {
    pub fn new() -> Result<Self, TaskError> {
        let conn = init_db()?;
        Ok(TaskManager { conn })
    }

    pub fn add_task(&self, task: Task) -> Result<(), TaskError> {
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

    pub fn list_tasks(&self, filter: StatusFilter) -> Result<Vec<Task>, TaskError> {
        let (sql, status_strings) = build_task_query(filter);
        let mut statement = self.conn.prepare(&sql)?;

        let map_row_to_task = |row: &rusqlite::Row| self.row_to_task(row);

        let results = if status_strings.is_empty() {
            statement.query_map([], map_row_to_task)?
        } else {
            let bindings: Vec<&dyn rusqlite::ToSql> =
                status_strings.iter().map(|status| status as _).collect();

            statement.query_map(&*bindings, map_row_to_task)?
        };

        let mut task_list = Vec::new();

        for result in results {
            task_list.push(result?);
        }

        Ok(task_list)
    }

    pub fn find_task_by_id(&self, short_id: &str) -> Result<Option<Task>, TaskError> {
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
                    .map(|id| &id[..8.min(id.len())])
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    pub fn delete_task_by_id(&self, id: &str) -> Result<bool, TaskError> {
        Ok(self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])? > 0)
    }

    pub fn update_task_status(
        &self,
        short_id: &str,
        new_status: Status,
    ) -> Result<bool, TaskError> {
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
                    .map(|id| &id[..8.min(id.len())])
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    pub fn update_name(&self, id: &str, name: &str) -> Result<bool, TaskError> {
        validate_task_name(name)?;
        Ok(self
            .conn
            .execute("UPDATE tasks SET name = ?1 WHERE id = ?2", [name, id])?
            > 0)
    }

    pub fn update_description(&self, id: &str, desc: &str) -> Result<bool, TaskError> {
        Ok(self.conn.execute(
            "UPDATE tasks SET description = ?1 WHERE id = ?2",
            [desc, id],
        )? > 0)
    }

    pub fn update_due(&self, id: &str, due: Option<DateTime<Utc>>) -> Result<bool, TaskError> {
        let s = due
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        Ok(self
            .conn
            .execute("UPDATE tasks SET due_date = ?1 WHERE id = ?2", [&s, id])?
            > 0)
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
        let status = std::str::FromStr::from_str(&status_str).unwrap_or(Status::Pending);
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
}

fn build_task_query(filter: StatusFilter) -> (String, Vec<String>) {
    let mut sql = String::from("SELECT id, date, name, description, status, due_date FROM tasks");

    let (where_clause, params) = filter.to_sql();
    if !where_clause.is_empty() {
        sql.push(' ');
        sql.push_str(&where_clause);
    }

    sql.push_str(" ORDER BY date DESC");
    (sql, params)
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

pub fn get_db_path() -> Result<PathBuf, TaskError> {
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
