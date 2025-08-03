use crate::types::TaskError;
use serde::{Deserialize, Serialize};
use std::io::Write as IoWrite;
use std::process::Command;
use std::{fs, io};
use tempfile::NamedTempFile;

/// A serialisable representation of a task for full-file editing.
/// `due` is a plain string, or None to clear the due date.
#[derive(Serialize, Deserialize)]
pub struct EditableTask {
    pub name: String,
    pub description: String,
    pub due: Option<String>,
}

impl EditableTask {
    /// Convert a `Task` into its editable representation.
    pub fn from_task(task: &crate::types::Task) -> Self {
        EditableTask {
            name: task.name.clone(),
            description: task.description.clone(),
            due: task
                .due_date
                .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

/// Launch the user’s editor with a TOML file representing the task.
/// Returns the edited representation, or a `TaskError` on failure.
pub fn edit_via_editor(task: &crate::types::Task) -> Result<EditableTask, TaskError> {
    let editable = EditableTask::from_task(task);

    let mut tmp = NamedTempFile::new().map_err(TaskError::Io)?;
    writeln!(
        tmp,
        "# Edit the fields below. Lines starting with '#' are ignored.\n\
         # Remove the 'due' key or leave it empty to clear the due date."
    )
    .map_err(TaskError::Io)?;

    // Write `name` normally. We use the `Debug` formatter to escape quotes.
    writeln!(tmp, "name = {:?}", editable.name).map_err(TaskError::Io)?;

    // Always write `description` as a triple-quoted multi-line string
    writeln!(tmp, "# Multi-line description. Leave blank if not needed.").map_err(TaskError::Io)?;

    writeln!(
        tmp,
        "description = \"\"\"\n{}\n\"\"\"",
        editable.description.trim_end()
    )
    .map_err(TaskError::Io)?;

    // For `due`, either write the string or an empty value
    match &editable.due {
        Some(d) => writeln!(tmp, "due = {:?}", d).map_err(TaskError::Io)?,
        None => writeln!(tmp, "due = \"\"").map_err(TaskError::Io)?,
    }

    tmp.flush().map_err(TaskError::Io)?;

    // Invoke editor
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    if let Err(err) = Command::new(&editor).arg(tmp.path()).status() {
        return Err(TaskError::Io(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to launch editor: {err}"),
        )));
    }

    // Read and parse edited file, stripping comment lines
    let contents = fs::read_to_string(tmp.path()).map_err(TaskError::Io)?;
    let cleaned: String = contents
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let mut edited: EditableTask = toml::from_str(&cleaned)
        .map_err(|e| TaskError::InvalidInput(format!("Failed to parse TOML: {e}")))?;

    edited.description = edited.description.trim().to_string();

    Ok(edited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Task;
    use chrono::{Local, TimeZone, Utc};

    #[test]
    fn test_editable_task_from_task_converts_to_local_time() {
        // Create a task with a specific UTC time
        let local_time = Local.with_ymd_and_hms(2025, 8, 15, 15, 30, 0).unwrap();
        let utc_time = local_time.with_timezone(&Utc);
        
        let task = Task {
            id: "test-id".to_string(),
            date: "2025-08-15 15:30:00".to_string(),
            name: "Test task".to_string(),
            description: "Test description".to_string(),
            status: crate::types::Status::Pending,
            due_date: Some(utc_time),
        };

        let editable = EditableTask::from_task(&task);
        
        // The editable task should show the original local time, not UTC
        assert_eq!(editable.due, Some("2025-08-15 15:30:00".to_string()));
        assert_eq!(editable.name, "Test task");
        assert_eq!(editable.description, "Test description");
    }

    #[test]
    fn test_editable_task_from_task_with_no_due_date() {
        let task = Task {
            id: "test-id".to_string(),
            date: "2025-08-15 15:30:00".to_string(),
            name: "Test task".to_string(),
            description: "Test description".to_string(),
            status: crate::types::Status::Pending,
            due_date: None,
        };

        let editable = EditableTask::from_task(&task);
        
        assert_eq!(editable.due, None);
        assert_eq!(editable.name, "Test task");
        assert_eq!(editable.description, "Test description");
    }

    #[test]
    fn test_editable_task_timezone_consistency() {
        // Test that the conversion maintains timezone consistency
        // What the user enters should be what they see in the editor
        let user_local_time = Local.with_ymd_and_hms(2025, 12, 25, 14, 30, 0).unwrap();
        let stored_utc_time = user_local_time.with_timezone(&Utc);
        
        let task = Task {
            id: "test-id".to_string(),
            date: "2025-12-25 14:30:00".to_string(),
            name: "Christmas task".to_string(),
            description: "".to_string(),
            status: crate::types::Status::Pending,
            due_date: Some(stored_utc_time),
        };

        let editable = EditableTask::from_task(&task);
        
        // Should show the original local time the user entered
        assert_eq!(editable.due, Some("2025-12-25 14:30:00".to_string()));
    }
}
