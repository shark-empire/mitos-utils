//! `head` -- print the first N lines of a file (default 10).
//! Supports `-n N`/`-N` and stdin.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use std::io::{self, BufRead, BufReader};

fn main() -> std::process::ExitCode {
    run("head", real_main)
}

fn real_main() -> AppResult<()> {
    let mut count: usize = 10;
    let mut files: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        if arg == "-n" {
            let n = args.next().ok_or_else(|| AppError::usage("option '-n' requires an argument"))?;
            count = n.parse().map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?;
        } else if let Some(n) = arg.strip_prefix('-').filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty()) {
            count = n.parse().unwrap_or(10);
        } else {
            files.push(arg);
        }
    }
    if files.is_empty() {
        files.push("-".to_string());
    }

    let multiple = files.len() > 1;
    let mut had_error = false;
    for (i, path) in files.iter().enumerate() {
        if multiple {
            if i > 0 {
                println!();
            }
            println!("==> {} <==", path);
        }
        let reader: Box<dyn BufRead> = if path == "-" {
            Box::new(BufReader::new(io::stdin()))
        } else {
            match std::fs::File::open(path) {
                Ok(f) => Box::new(BufReader::new(f)),
                Err(err) => {
                    error_path("head", path, err);
                    had_error = true;
                    continue;
                }
            }
        };
        for line in reader.lines().take(count).flatten() {
            println!("{}", line);
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
