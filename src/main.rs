use std::io;

mod cli;
mod commands;
mod database;
mod display;
mod editor;
mod help;
mod paging;
mod types;
mod utils;

// Re-export types for public use
pub use types::*;
use database::TaskManager;
use commands::{execute_command, estimated_lines};
use paging::{PagerConfig, init as pager_init};

fn main() -> io::Result<()> {
    help::handle_flag_help()?;

    let command = cli::parse_command();

    let manager = match TaskManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to initialize task manager: {}", e);
            return Ok(());
        }
    };

    let line_estimate = estimated_lines(&command, &manager);

    pager_init(PagerConfig {
        lines: line_estimate,
        needs_color: true,
    })?;

    if let Err(e) = execute_command(&manager, command) {
        eprintln!("{}", e);
    }

    Ok(())
}