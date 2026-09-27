//! `sort` -- sort lines of text. Supports `-r` (reverse), `-n`
//! (numeric), `-u` (unique), `-k N` (sort by field N instead of the
//! whole line -- single field only, not an `N,M` range), `-t DELIM`
//! (field delimiter for `-k`; default is any whitespace run), and
//! stdin.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, stdout_writer};
use std::io::{self, BufRead, BufReader, Write};

pub const USAGE: &str =
    "sort [-r] [-n] [-u] [-k N] [-t DELIM] [FILE...] -- sort lines (optionally by field N)";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut reverse = false;
    let mut numeric = false;
    let mut unique = false;
    let mut key_field: Option<usize> = None;
    let mut delimiter: Option<char> = None;
    let mut files: Vec<String> = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-r" => reverse = true,
            "-n" => numeric = true,
            "-u" => unique = true,
            "-k" => {
                let n = args
                    .next()
                    .ok_or_else(|| AppError::usage("option '-k' requires an argument"))?;
                let first_field = n.split(',').next().unwrap_or(&n);
                key_field = Some(
                    first_field
                        .parse()
                        .map_err(|_| AppError::usage(format!("invalid key: '{}'", n)))?,
                );
            }
            "-t" => {
                let d = args
                    .next()
                    .ok_or_else(|| AppError::usage("option '-t' requires an argument"))?;
                delimiter = d.chars().next();
            }
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

    // Extract the sort key for a line: the whole line by default, or
    // field `key_field` (1-indexed) split on `delimiter` (default:
    // any whitespace run) when `-k` was given.
    let key_of = |line: &str| -> String {
        match key_field {
            None => line.to_string(),
            Some(field) if field >= 1 => {
                let parts: Vec<&str> = match delimiter {
                    Some(d) => line.split(d).collect(),
                    None => line.split_whitespace().collect(),
                };
                parts.get(field - 1).copied().unwrap_or("").to_string()
            }
            _ => line.to_string(),
        }
    };

    // Decorate-sort-undecorate: compute each line's sort key --
    // and, for `-n`, its parsed numeric value -- exactly once up
    // front, instead of recomputing (and, in the numeric case,
    // reparsing a float) it on every comparison a naive `sort_by`
    // makes. For N lines that's O(N) key extractions instead of
    // O(N log N) of them, which matters a lot once N gets large.
    if numeric {
        let mut decorated: Vec<(f64, String)> = lines
            .into_iter()
            .map(|line| {
                let key: f64 = key_of(&line).trim().parse().unwrap_or(0.0);
                (key, line)
            })
            .collect();
        decorated.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        lines = decorated.into_iter().map(|(_, line)| line).collect();
    } else {
        let mut decorated: Vec<(String, String)> = lines
            .into_iter()
            .map(|line| (key_of(&line), line))
            .collect();
        decorated.sort_by(|a, b| a.0.cmp(&b.0));
        lines = decorated.into_iter().map(|(_, line)| line).collect();
    }
    if reverse {
        lines.reverse();
    }
    if unique {
        lines.dedup();
    }

    let mut out = stdout_writer();
    for line in &lines {
        let _ = writeln!(out, "{}", line);
    }
    let _ = out.flush();

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
