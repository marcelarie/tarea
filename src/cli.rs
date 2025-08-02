use crate::types::{Status, TaskCommand, EditField};
use crate::utils::parse_due_date;
use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use std::str::FromStr;

const DYNAMIC_COMPLETE_BASH: &str = r#"
if ! declare -f _tarea_clap >/dev/null ; then
    eval "$(declare -f _tarea | sed "s/^_tarea/_tarea_clap/")"
fi

_tarea() {
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    local filter=""

    case "$prev" in
        --done)
            filter="--filter=standby,pending"
            ;;
        --pending)
            filter="--filter=done,standby"
            ;;
        --standby)
            filter="--filter=done,pending"
            ;;
        --show|--edit|-e|--delete)
            # No filter, allow matching any task
            filter="--filter=done,pending,standby"
            ;;
        *)
            _tarea_clap "$@"
            return
            ;;
    esac

    COMPREPLY=( $(compgen -W "$(tarea --ids --short $filter 2>/dev/null)" \
                      -- "${COMP_WORDS[COMP_CWORD]}") )
}
"#;

const DYNAMIC_COMPLETE_FISH: &str = r#"
function __tarea_status_complete
    set cmd (commandline -opc)
    set filter ""
    for arg in $cmd
        switch $arg
            case --done
                set filter "--filter=standby,pending"
            case --pending
                set filter "--filter=done,standby"
            case --standby
                set filter "--filter=done,pending"
            case --show --edit --delete -e
                set filter "--filter=done,pending,standby"
        end
    end
    if test -n "$filter"
        tarea --ids --short $filter
    else

    end
end

complete -r -f -c tarea -l done -a '(__tarea_status_complete)' -d 'Mark tasks as done'
complete -r -f -c tarea -l pending -a '(__tarea_status_complete)' -d 'Mark tasks as pending'
complete -r -f -c tarea -l standby -a '(__tarea_status_complete)' -d 'Mark tasks as standby'
complete -r -f -c tarea -l show -a '(__tarea_status_complete)' -d 'Show specific task by ID'
complete -r -f -c tarea -l edit -a '(__tarea_status_complete)' -d 'Edit task'
complete -r -f -c tarea -l delete -a '(__tarea_status_complete)' -d 'Delete a task by ID'
"#;

pub fn build_cli() -> Command {
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
            Arg::new("delete")
                .long("delete")
                .help("Delete a task by ID or list index")
                .value_name("TASK")
                .num_args(1),
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
            Arg::new("filter")
                .long("filter")
                .num_args(1)
                .value_name("STATUS[,STATUS...]")
                .help("Only show tasks with any of the given statuses (used with --ids)"),
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

pub fn parse_command() -> TaskCommand {
    let matches = build_cli().get_matches();

    if matches.get_flag("delete-database") {
        return TaskCommand::DeleteDatabase;
    }

    if matches.get_flag("ids") && !matches.contains_id("task") {
        let short = matches.get_flag("short");
        let filter = matches
            .get_one::<String>("filter")
            .map(|status| {
                status
                    .split(',')
                    .filter_map(|st| Status::from_str(st.trim()).ok())
                    .collect()
            })
            .unwrap_or_default();

        return TaskCommand::Ids {
            short_only: short,
            filter,
        };
    }

    if matches.contains_id("name") && !matches.contains_id("task") {
        let id_opt = matches.get_one::<String>("name").cloned();
        let status = status_flag(&matches).map(|(s, _)| s);

        if let Some(id) = id_opt {
            return TaskCommand::ShowName {
                id_or_index: id,
                status,
            };
        }

        return TaskCommand::ListNames {
            show_all: matches.get_flag("all"),
            status,
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
        return parse_edit_command(&matches, id_val);
    }

    if let Some(task_id) = matches.get_one::<String>("show") {
        return TaskCommand::Show {
            id: task_id.clone(),
        };
    }

    if let Some(task_id) = matches.get_one::<String>("delete") {
        let status = status_flag(&matches).map(|(s, _)| s);
        return TaskCommand::Delete {
            id_or_index: task_id.clone(),
            status,
        };
    }

    if let Some(shell) = matches.get_one::<String>("completions") {
        return TaskCommand::Completions {
            shell: shell.clone(),
            dynamic_bash: DYNAMIC_COMPLETE_BASH.to_string(),
            dynamic_fish: DYNAMIC_COMPLETE_FISH.to_string(),
        };
    }

    if let Some(name) = get_task_name(&matches) {
        return parse_add_command(&matches, name);
    }

    let show_descriptions = get_show_descriptions(&matches);
    let show_all = matches.get_flag("all");

    TaskCommand::List {
        status: None,
        show_all,
        show_descriptions,
    }
}

fn parse_edit_command(matches: &clap::ArgMatches, id_val: &str) -> TaskCommand {
    let has_due = matches.contains_id("due-date");
    let has_desc = matches.contains_id("description");
    let explicit_name = matches.contains_id("name")
        || matches
            .get_many::<String>("task")
            .map(|vals| !vals.collect::<Vec<_>>().is_empty())
            .unwrap_or(false);

    let should_open_editor = !has_due && !has_desc && !explicit_name;

    if should_open_editor {
        return TaskCommand::EditWithEditor {
            id_or_index: id_val.to_string(),
        };
    }

    if let Some(due_vals) = matches.get_many::<String>("due-date") {
        let raw = due_vals
            .map(|status| status.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let new_due = match parse_due_date(&raw) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        return TaskCommand::Edit {
            id_or_index: id_val.to_string(),
            field: EditField::DueDate(new_due),
        };
    }

    if let Some(desc_vals) = matches.get_many::<String>("description") {
        let desc = desc_vals
            .map(|status| status.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if desc.is_empty() {
            eprintln!("No new description supplied");
            std::process::exit(1);
        }
        return TaskCommand::Edit {
            id_or_index: id_val.to_string(),
            field: EditField::Description(desc),
        };
    }

    let new_name = get_edit_name(matches);
    TaskCommand::Edit {
        id_or_index: id_val.to_string(),
        field: EditField::Name(new_name),
    }
}

fn get_edit_name(matches: &clap::ArgMatches) -> String {
    if let Some(first) = matches.get_one::<String>("name") {
        let mut name_clone = first.clone();
        if let Some(rest) = matches.get_many::<String>("task") {
            let tail = rest
                .map(|status| status.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            if !tail.is_empty() {
                name_clone.push(' ');
                name_clone.push_str(&tail);
            }
        }
        name_clone
    } else {
        matches
            .get_many::<String>("task")
            .map(|vals| {
                vals.map(|status| status.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|status| !status.is_empty())
            .unwrap_or_else(|| {
                eprintln!("No new name supplied");
                std::process::exit(1);
            })
    }
}

fn parse_add_command(matches: &clap::ArgMatches, name: String) -> TaskCommand {
    let description = get_description(matches);
    let due_date = get_due_date(matches);

    TaskCommand::Add {
        name,
        description,
        due_date,
    }
}

fn get_task_name(matches: &clap::ArgMatches) -> Option<String> {
    matches
        .get_many::<String>("task")
        .map(|vals| {
            vals.map(|status| status.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|status| !status.is_empty())
}

fn get_description(matches: &clap::ArgMatches) -> Option<String> {
    if let Some(desc_vals) = matches.get_many::<String>("description") {
        let desc_text = desc_vals.map(|status| status.as_str()).collect::<Vec<_>>();
        if desc_text.is_empty() {
            None
        } else {
            Some(desc_text.join(" "))
        }
    } else {
        None
    }
}

fn get_due_date(matches: &clap::ArgMatches) -> Option<DateTime<Utc>> {
    if let Some(date_vals) = matches.get_many::<String>("due-date") {
        let date_str = date_vals
            .map(|status| status.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        match parse_due_date(&date_str) {
            Ok(dt) => Some(dt),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    }
}

fn get_show_descriptions(matches: &clap::ArgMatches) -> bool {
    if let Some(desc_vals) = matches.get_many::<String>("description") {
        desc_vals.collect::<Vec<_>>().is_empty()
    } else {
        matches.contains_id("description")
    }
}