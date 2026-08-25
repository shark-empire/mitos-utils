//! `cut` -- extract fields from each line. Supports `-d DELIM`
//! (default tab) and `-f LIST` (comma-separated field numbers/ranges
//! like `1,3-5`).

use mitos_utils::common::errors::{run, AppError, AppResult};
use std::io::{self, BufRead};

fn main() -> std::process::ExitCode {
    run("cut", real_main)
}

fn real_main() -> AppResult<()> {
    let mut delimiter = '\t';
    let mut fields_spec: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-d" => {
                let d = args.next().ok_or_else(|| AppError::usage("option '-d' requires an argument"))?;
                delimiter = d.chars().next().unwrap_or('\t');
            }
            "-f" => {
                fields_spec = Some(args.next().ok_or_else(|| AppError::usage("option '-f' requires an argument"))?);
            }
            _ => files.push(arg),
        }
    }
    let fields_spec = fields_spec.ok_or_else(|| AppError::usage("you must specify a list of fields with -f"))?;
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

fn parse_field_list(spec: &str) -> AppResult<Vec<usize>> {
    let mut out = Vec::new();
    for part in spec.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            let start: usize = a.parse().map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?;
            let end: usize = b.parse().map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?;
            out.extend(start..=end);
        } else {
            out.push(part.parse().map_err(|_| AppError::usage(format!("invalid field list: '{}'", spec)))?);
        }
    }
    Ok(out)
}
