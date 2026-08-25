//! `uniq` -- collapse adjacent duplicate lines. Supports `-c`
//! (prefix with a count) and `-d` (only show lines that had
//! duplicates).

use mitos_utils::common::errors::{run, AppError, AppResult};
use std::io::{self, BufRead};

fn main() -> std::process::ExitCode {
    run("uniq", real_main)
}

fn real_main() -> AppResult<()> {
    let mut show_count = false;
    let mut only_duplicates = false;
    let mut file: Option<String> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-c" => show_count = true,
            "-d" => only_duplicates = true,
            _ => file = Some(arg),
        }
    }

    let lines: Vec<String> = match file {
        Some(path) if path != "-" => {
            let content = std::fs::read_to_string(&path).map_err(|e| AppError::new(format!("{}: {}", path, e)))?;
            content.lines().map(String::from).collect()
        }
        _ => io::stdin().lock().lines().flatten().collect(),
    };

    let mut i = 0;
    while i < lines.len() {
        let current = &lines[i];
        let mut count = 1;
        while i + count < lines.len() && lines[i + count] == *current {
            count += 1;
        }
        if !only_duplicates || count > 1 {
            if show_count {
                println!("{:>7} {}", count, current);
            } else {
                println!("{}", current);
            }
        }
        i += count;
    }
    Ok(())
}
