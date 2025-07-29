use crate::TaskError;
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
    pub fn from_task(task: &crate::Task) -> Self {
        EditableTask {
            name: task.name.clone(),
            description: task.description.clone(),
            due: task
                .due_date
                .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

/// Launch the user’s editor with a TOML file representing the task.
/// Returns the edited representation, or a `TaskError` on failure.
pub fn edit_via_editor(task: &crate::Task) -> Result<EditableTask, TaskError> {
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
    writeln!(
        tmp,
        "description = \"\"\"\n{}\n\"\"\"",
        editable.description
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
    toml::from_str(&cleaned)
        .map_err(|e| TaskError::InvalidInput(format!("Failed to parse TOML: {e}")))
}
