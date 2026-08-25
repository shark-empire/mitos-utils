//! `sort` -- sort lines of text. Supports `-r` (reverse), `-n`
//! (numeric), `-u` (unique), and stdin.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use std::io::{self, BufRead, BufReader};

fn main() -> std::process::ExitCode {
    run("sort", real_main)
}

fn real_main() -> AppResult<()> {
    let mut reverse = false;
    let mut numeric = false;
    let mut unique = false;
    let mut files: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-r" => reverse = true,
            "-n" => numeric = true,
            "-u" => unique = true,
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut lines: Vec<String> = Vec::new();
    let mut had_error = false;
    for path in &files {
        let reader: Box<dyn BufRead> = if path == "-" {
            Box::new(BufReader::new(io::stdin()))
        } else {
            match std::fs::File::open(path) {
                Ok(f) => Box::new(BufReader::new(f)),
                Err(err) => {
                    error_path("sort", path, err);
                    had_error = true;
                    continue;
                }
            }
        };
        lines.extend(reader.lines().flatten());
    }

    if numeric {
        lines.sort_by(|a, b| {
            let na: f64 = a.trim().parse().unwrap_or(0.0);
            let nb: f64 = b.trim().parse().unwrap_or(0.0);
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort();
    }
    if reverse {
        lines.reverse();
    }
    if unique {
        lines.dedup();
    }

    for line in &lines {
        println!("{}", line);
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
