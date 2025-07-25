use std::io;

use clap::{Arg, Command};

fn print_due_help() -> io::Result<()> {
    let mut cmd = Command::new("--due")
        .about("Set a due date for the task")
        .override_usage("tarea [TASK]... --due <DATE>...")
        .arg(
            Arg::new("date")
                .help("Natural-language (today, tomorrow), relative (2h, 45m) or absolute YYYY-MM-DD[ HH:MM[:SS]] date")
                .required(true)
                .num_args(1..)
                .value_name("DATE"),
        )
        .after_help(
"Format examples:
  today                today 23:59:59
  tomorrow             tomorrow 23:59:59
  2h                   N hours from now   (e.g. 2h, 6h)
  30m                  N minutes from now (e.g. 30m, 90m)
  2025-08-01           YYYY-MM-DD
  2025-08-01 17:00     YYYY-MM-DD HH:MM
  2025-08-01 17:00:30  YYYY-MM-DD HH:MM:SS

Command Examples:
  tarea Pay rent       --due today
  tarea Water plants   --due 2h
  tarea Release v1.2   --due 2025-08-01 17:00
"
        ).color(clap::ColorChoice::Auto);

    cmd.print_help()?;
    println!();
    Ok(())
}

pub fn handle_flag_help() -> io::Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let has_any_help_argument = raw.iter().any(|s| s == "--help" || s == "-h");

    if !has_any_help_argument {
        return Ok(());
    }

    let rest: Vec<&str> = raw
        .iter()
        .filter(|s| *s != "--help" && *s != "-h")
        .map(String::as_str)
        .collect();

    match rest.as_slice() {
        ["--due"] => {
            print_due_help()?;
            std::process::exit(0);
        }
        [] => Ok(()),
        _ => {
            eprintln!(
                "error: '--help' may only be combined with one supported flag (e.g. '--help --due')"
            );
            std::process::exit(2);
        }
    }
}
