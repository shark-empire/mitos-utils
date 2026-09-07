//! `uniq` -- collapse adjacent duplicate lines. Supports `-c`
//! (prefix with a count) and `-d` (only show lines that had
//! duplicates).
//!
//! Streams its input line by line instead of buffering the whole
//! file into memory first: `uniq` only ever needs to compare a line
//! against the one immediately before it, so there's no reason to
//! hold the rest of the file in memory, or wait for EOF before
//! producing any output, the way collecting into a `Vec<String>` up
//! front used to.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::stdout_writer;
use std::io::{self, BufRead, BufReader, Write};

pub const USAGE: &str = "uniq [-c] [-d] [FILE] -- collapse adjacent duplicate lines";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut show_count = false;
    let mut only_duplicates = false;
    let mut file: Option<String> = None;

    for arg in args {
        match arg.as_str() {
            "-c" => show_count = true,
            "-d" => only_duplicates = true,
            _ => file = Some(arg),
        }
    }

    let reader: Box<dyn BufRead> = match &file {
        Some(path) if path != "-" => Box::new(BufReader::new(
            std::fs::File::open(path).map_err(|e| AppError::new(format!("{}: {}", path, e)))?,
        )),
        _ => Box::new(BufReader::new(io::stdin())),
    };

    let mut out = stdout_writer();
    let mut current: Option<String> = None;
    let mut count: u64 = 0;

    for line in reader.lines() {
        let Ok(line) = line else { break };
        match &current {
            Some(prev) if *prev == line => count += 1,
            Some(prev) => {
                emit_line(&mut out, only_duplicates, show_count, prev, count);
                current = Some(line);
                count = 1;
            }
            None => {
                current = Some(line);
                count = 1;
            }
        }
    }
    if let Some(prev) = &current {
        emit_line(&mut out, only_duplicates, show_count, prev, count);
    }

    let _ = out.flush();
    Ok(())
}

fn emit_line(
    out: &mut impl Write,
    only_duplicates: bool,
    show_count: bool,
    line: &str,
    count: u64,
) {
    if !only_duplicates || count > 1 {
        if show_count {
            let _ = writeln!(out, "{:>7} {}", count, line);
        } else {
            let _ = writeln!(out, "{}", line);
        }
    }
}
