//! `grep` -- print lines matching a pattern.
//!
//! Supports `-E` (regex, default), `-F` (fixed string), `-i`
//! (case-insensitive), `-v` (invert match), `-n` (print line
//! numbers), and plain substring/basic patterns.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, stdout_writer};
use std::io::{self, BufRead, BufReader, Write};

pub const USAGE: &str = "grep [-E|-F] [-i] [-v] [-n] PATTERN [FILE...] -- print matching lines";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut fixed_string = false;
    let mut ignore_case = false;
    let mut invert = false;
    let mut show_line_numbers = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-E" => fixed_string = false, // Default is regex
            "-F" => fixed_string = true,
            "-i" => ignore_case = true,
            "-v" => invert = true,
            "-n" => show_line_numbers = true,
            _ => rest.push(arg),
        }
    }
    rest.extend(forced);
    if rest.is_empty() {
        return Err(AppError::usage(
            "usage: grep [-E|-F] [-i] [-v] [-n] PATTERN [FILE]...",
        ));
    }

    let pattern = rest.remove(0);
    let files = rest;
    let sources: Vec<String> = if files.is_empty() {
        vec!["-".to_string()]
    } else {
        files
    };
    let multiple = sources.len() > 1;

    // Pre-compile pattern for regex matching
    let pattern_chars: Vec<char> = if ignore_case {
        pattern.to_lowercase().chars().collect()
    } else {
        pattern.chars().collect()
    };

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
            let is_match = if fixed_string {
                if ignore_case {
                    line.to_lowercase().contains(&pattern.to_lowercase())
                } else {
                    line.contains(&pattern)
                }
            } else {
                let text_chars: Vec<char> = if ignore_case {
                    line.to_lowercase().chars().collect()
                } else {
                    line.chars().collect()
                };
                match_regex(&pattern_chars, &text_chars)
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

// --- Zero-dependency Regex Engine ---

fn match_here(regexp: &[char], text: &[char]) -> bool {
    if regexp.is_empty() {
        return true;
    }

    let mut current = regexp[0];
    let mut next_is_star = false;
    let mut rest_regexp = &regexp[1..];

    if !rest_regexp.is_empty() && rest_regexp[0] == '*' {
        next_is_star = true;
        rest_regexp = &rest_regexp[1..];
    } else if current == '\\' && !rest_regexp.is_empty() {
        current = rest_regexp[0];
        rest_regexp = &rest_regexp[1..];
        if !rest_regexp.is_empty() && rest_regexp[0] == '*' {
            next_is_star = true;
            rest_regexp = &rest_regexp[1..];
        }
    }

    if next_is_star {
        return match_star(current, rest_regexp, text);
    }

    if current == '$' && rest_regexp.is_empty() {
        return text.is_empty();
    }

    if !text.is_empty() && (current == '.' || current == text[0]) {
        return match_here(rest_regexp, &text[1..]);
    }

    false
}

fn match_star(c: char, regexp: &[char], text: &[char]) -> bool {
    let mut t = text;
    loop {
        if match_here(regexp, t) {
            return true;
        }
        if t.is_empty() || (c != '.' && c != t[0]) {
            return false;
        }
        t = &t[1..];
    }
}

fn match_regex(regexp: &[char], text: &[char]) -> bool {
    if !regexp.is_empty() && regexp[0] == '^' {
        return match_here(&regexp[1..], text);
    }
    let mut t = text;
    loop {
        if match_here(regexp, t) {
            return true;
        }
        if t.is_empty() {
            return false;
        }
        t = &t[1..];
    }
}
