//! `wc` -- count lines, words, and bytes. Supports `-l`, `-w`, `-c`
//! individually (default: all three) and multiple files with a
//! total line.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use std::io::{self, Read};

struct Counts {
    lines: u64,
    words: u64,
    bytes: u64,
}

fn main() -> std::process::ExitCode {
    run("wc", real_main)
}

fn real_main() -> AppResult<()> {
    let mut show_lines = false;
    let mut show_words = false;
    let mut show_bytes = false;
    let mut files: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-l" => show_lines = true,
            "-w" => show_words = true,
            "-c" => show_bytes = true,
            _ => files.push(arg),
        }
    }
    if !show_lines && !show_words && !show_bytes {
        show_lines = true;
        show_words = true;
        show_bytes = true;
    }
    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut total = Counts { lines: 0, words: 0, bytes: 0 };
    let multiple = files.len() > 1;
    let mut had_error = false;

    for path in &files {
        let content = if path == "-" {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf).map_err(AppError::from)?;
            buf
        } else {
            match std::fs::read(path) {
                Ok(b) => b,
                Err(err) => {
                    error_path("wc", path, err);
                    had_error = true;
                    continue;
                }
            }
        };
        let counts = count(&content);
        print_counts(&counts, show_lines, show_words, show_bytes, path, files.len() > 1 || path != "-");
        total.lines += counts.lines;
        total.words += counts.words;
        total.bytes += counts.bytes;
    }

    if multiple {
        print_counts(&total, show_lines, show_words, show_bytes, "total", true);
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn count(content: &[u8]) -> Counts {
    let lines = content.iter().filter(|&&b| b == b'\n').count() as u64;
    let words = content
        .split(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
        .filter(|w| !w.is_empty())
        .count() as u64;
    Counts { lines, words, bytes: content.len() as u64 }
}

fn print_counts(c: &Counts, l: bool, w: bool, b: bool, label: &str, show_label: bool) {
    let mut parts = Vec::new();
    if l {
        parts.push(format!("{:>7}", c.lines));
    }
    if w {
        parts.push(format!("{:>7}", c.words));
    }
    if b {
        parts.push(format!("{:>7}", c.bytes));
    }
    if show_label {
        println!("{} {}", parts.join(""), label);
    } else {
        println!("{}", parts.join(""));
    }
}
