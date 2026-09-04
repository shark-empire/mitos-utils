//! `cat` -- concatenate files to stdout. Supports `-n`/`--number`,
//! `-b`/`--number-nonblank`, reading multiple files, and `-` for
//! stdin. Line-oriented (like the rest of this crate's text tools),
//! not binary-safe -- see docs/compatibility.md.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub const USAGE: &str = "cat [-n|-b] [FILE...] -- concatenate files to stdout";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut number_all = false;
    let mut number_nonblank = false;
    let mut files: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-n" | "--number" => number_all = true,
            "-b" | "--number-nonblank" => number_nonblank = true,
            "-" => files.push(arg),
            _ if arg.starts_with("--") => {
                return Err(AppError::usage(format!("unrecognized option '{}'", arg)))
            }
            _ if arg.starts_with('-') && arg.len() > 1 => {
                return Err(AppError::usage(format!(
                    "invalid option -- '{}'",
                    &arg[1..]
                )))
            }
            _ => files.push(arg),
        }
    }
    files.extend(forced);
    if files.is_empty() {
        files.push("-".to_string());
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut line_no: u64 = 1;
    let mut had_error = false;

    for path in &files {
        let reader: Box<dyn BufRead> = if path == "-" {
            Box::new(BufReader::new(io::stdin()))
        } else {
            match File::open(path) {
                Ok(f) => Box::new(BufReader::new(f)),
                Err(err) => {
                    error_path("cat", path, err);
                    had_error = true;
                    continue;
                }
            }
        };
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(err) => {
                    error_path("cat", path, err);
                    had_error = true;
                    break;
                }
            };
            if number_nonblank {
                if line.is_empty() {
                    let _ = writeln!(out);
                } else {
                    let _ = writeln!(out, "{:>6}\t{}", line_no, line);
                    line_no += 1;
                }
            } else if number_all {
                let _ = writeln!(out, "{:>6}\t{}", line_no, line);
                line_no += 1;
            } else {
                let _ = writeln!(out, "{}", line);
            }
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
