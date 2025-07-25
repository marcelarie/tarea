use atty::Stream;
use colored::control;
use pager::Pager;
use std::io;
use terminal_size::{Height, terminal_size};

pub struct PagerConfig {
    /// Rough number of lines that your program is going to print.
    pub lines: usize,
    /// Set to `true` when the output contains ANSI colour escapes.
    pub needs_color: bool,
}

pub fn init(cfg: PagerConfig) -> io::Result<()> {
    if !atty::is(Stream::Stdout) {
        return Ok(());
    }

    let term_height = terminal_size()
        .map(|(_, Height(h))| h as usize)
        .unwrap_or(24);

    let should_not_use_pager = cfg.lines <= term_height;
    if should_not_use_pager {
        return Ok(());
    }

    if cfg.needs_color {
        control::set_override(true);
    }

    Pager::with_default_pager("less -FRX").setup();

    Ok(())
}
