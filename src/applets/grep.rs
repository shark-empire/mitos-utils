//! `grep` -- print lines matching a pattern. Supports `-i`
//! (case-insensitive), `-v` (invert match), `-n` (print line
//! numbers), and plain substring/basic patterns (no full regex --
//! see docs/compatibility.md).

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, stdout_writer};
use std::io::{self, BufRead, BufReader, Write};

pub const USAGE: &str = "grep [-i] [-v] [-n] PATTERN [FILE...] -- print matching lines";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut ignore_case = false;
    let mut invert = false;
    let mut show_line_numbers = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-i" => ignore_case = true,
            "-v" => invert = true,
            "-n" => show_line_numbers = true,
            _ => rest.push(arg),
        }
    }
    rest.extend(forced);
    if rest.is_empty() {
        return Err(AppError::usage(
            "usage: grep [-i] [-v] [-n] PATTERN [FILE]...",
        ));
    }

    let pattern = rest.remove(0);
    let pattern_cmp = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.clone()
    };
    let files = rest;
    let sources: Vec<String> = if files.is_empty() {
        vec!["-".to_string()]
    } else {
        files
    };
    let multiple = sources.len() > 1;

    let mut out = stdout_writer();
    let mut had_error = false;
    let mut matched_any = false;

    for path in &sources {
        let reader: Box<dyn BufRead> = if path == "-" {
            Box::new(BufReader::new(io::stdin()))
        } else {
            match std::fs::File::open(path) {
                Ok(f) => Box::new(BufReader::new(f)),
                Err(err) => {
                    error_path("grep", path, err);
                    had_error = true;
                    continue;
                }
            }
        };

        for (i, line) in reader.lines().flatten().enumerate() {
            // Case-sensitive matching (the common case) borrows
            // `line` directly instead of cloning it just to call
            // `.contains()` -- the clone bought nothing here, since
            // `str::contains` never needed an owned copy in the
            // first place. Case-insensitive matching still needs one
            // `to_lowercase()` allocation; there's no folding it away
            // without full Unicode case-folding support.
            let is_match = if ignore_case {
                line.to_lowercase().contains(&pattern_cmp)
            } else {
                line.contains(&pattern_cmp)
            };
            if is_match != invert {
                matched_any = true;
                if multiple {
                    let _ = write!(out, "{}:", path);
                }
                if show_line_numbers {
                    let _ = writeln!(out, "{}:{}", i + 1, line);
                } else {
                    let _ = writeln!(out, "{}", line);
                }
            }
        }
    }

    let _ = out.flush();
    if had_error {
        Err(AppError::silent(2))
    } else if !matched_any {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
