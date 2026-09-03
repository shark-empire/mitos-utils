//! `clear` -- clear the terminal screen via the ANSI escape sequence
//! (no terminfo database lookup -- see docs/compatibility.md).

use crate::common::errors::AppResult;

pub const USAGE: &str = "clear -- clear the terminal screen";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    print!("\x1b[2J\x1b[H");
    Ok(())
}
