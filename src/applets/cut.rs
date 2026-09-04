//! `cut` -- extract fields from each line. Supports `-d DELIM`
//! (default tab) and `-f LIST` (comma-separated field numbers/ranges
//! like `1,3-5`).

use crate::common::errors::{AppError, AppResult};
use std::io::{self, BufRead};

pub const USAGE: &str = "cut -d DELIM -f LIST [FILE...] -- extract fields from each line";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut delimiter = '\t';
    let mut fields_spec: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-d" => {
                let d = args
                    .next()
                    .ok_or_else(|| AppError::usage("option '-d' requires an argument"))?;
                delimiter = d.chars().next().unwrap_or('\t');
            }
            "-f" => {
                fields_spec = Some(
                    args.next()
                        .ok_or_else(|| AppError::usage("option '-f' requires an argument"))?,
                );
            }
            _ => files.push(arg),
        }
    }
    let fields_spec =
        fields_spec.ok_or_else(|| AppError::usage("you must specify a list of fields with -f"))?;
    let fields = parse_field_list(&fields_spec)?;

    if files.is_empty() {
        files.push("-".to_string());
    }

    for path in &files {
        let reader: Box<dyn BufRead> = if path == "-" {
            Box::new(io::BufReader::new(io::stdin()))
        } else {
            Box::new(io::BufReader::new(
                std::fs::File::open(path).map_err(|e| AppError::new(format!("{}: {}", path, e)))?,
            ))
        };
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split(delimiter).collect();
            let selected: Vec<&str> = fields
                .iter()
                .filter_map(|&f| parts.get(f.saturating_sub(1)).copied())
                .collect();
            println!("{}", selected.join(&delimiter.to_string()));
        }
    }
    Ok(())
}

/// Parse a `-f` field-list spec (`"1,3-5"`) into a flat list of
/// 1-indexed field numbers. Exposed as a standalone function so it
/// can be exercised directly by
/// `fuzz/fuzz_targets/cut_field_list.rs` -- untrusted, hand-written
/// range-parsing logic like this is exactly the kind of thing worth
/// fuzzing (integer overflow on huge numbers, reversed ranges,
/// empty segments, and similar edge cases are easy to miss by hand).
pub fn parse_field_list(spec: &str) -> AppResult<Vec<usize>> {
    let mut out = Vec::new();
    for part in spec.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            let start: usize = a
                .parse()
                .map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?;
            let end: usize = b
                .parse()
                .map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?;
            if start > end {
                return Err(AppError::usage(format!(
                    "invalid decreasing range: '{}'",
                    part
                )));
            }
            out.extend(start..=end);
        } else {
            out.push(
                part.parse()
                    .map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?,
            );
        }
    }
    Ok(out)
}
