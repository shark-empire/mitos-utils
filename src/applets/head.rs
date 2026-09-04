//! `head` -- print the first N lines of a file (default 10).
//! Supports `-n N`/`-N` (lines), `-c N` (bytes -- binary-safe, unlike
//! this crate's line-mode text tools), and stdin.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::io::{self, BufRead, BufReader, Read};

enum Mode {
    Lines(usize),
    Bytes(usize),
}

pub const USAGE: &str =
    "head [-n N] [-c N] [FILE...] -- print the first N lines (default 10) or bytes";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut mode = Mode::Lines(10);
    let mut files: Vec<String> = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if arg == "-n" {
            let n = args
                .next()
                .ok_or_else(|| AppError::usage("option '-n' requires an argument"))?;
            mode = Mode::Lines(
                n.parse()
                    .map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?,
            );
        } else if arg == "-c" {
            let n = args
                .next()
                .ok_or_else(|| AppError::usage("option '-c' requires an argument"))?;
            mode = Mode::Bytes(
                n.parse()
                    .map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?,
            );
        } else if let Some(n) = arg
            .strip_prefix('-')
            .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
        {
            mode = Mode::Lines(n.parse().unwrap_or(10));
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
        let opened: Box<dyn Read> = if path == "-" {
            Box::new(io::stdin())
        } else {
            match std::fs::File::open(path) {
                Ok(f) => Box::new(f),
                Err(err) => {
                    error_path("head", path, err);
                    had_error = true;
                    continue;
                }
            }
        };

        match mode {
            Mode::Lines(count) => {
                for line in BufReader::new(opened).lines().take(count).flatten() {
                    println!("{}", line);
                }
            }
            Mode::Bytes(count) => {
                let stdout = io::stdout();
                let mut out = stdout.lock();
                let _ = io::copy(&mut opened.take(count as u64), &mut out);
            }
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
