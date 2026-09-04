//! `wc` -- count lines, words, and bytes. Supports `-l`, `-w`, `-c`
//! individually (default: all three) and multiple files with a
//! total line.
//!
//! Reads in fixed-size chunks rather than loading the whole file
//! into memory, so this stays cheap on multi-GB files -- word-break
//! state is carried across chunk boundaries so counts come out the
//! same either way.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::io::{self, Read};

const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Default, Clone, Copy)]
struct Counts {
    lines: u64,
    words: u64,
    bytes: u64,
}

pub const USAGE: &str = "wc [-l] [-w] [-c] [FILE...] -- count lines/words/bytes";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut show_lines = false;
    let mut show_words = false;
    let mut show_bytes = false;
    let mut files: Vec<String> = Vec::new();

    for arg in args {
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

    let mut total = Counts::default();
    let multiple = files.len() > 1;
    let mut had_error = false;

    for path in &files {
        let counts = if path == "-" {
            match count_stream(io::stdin().lock()) {
                Ok(c) => c,
                Err(err) => {
                    error_path("wc", path, err);
                    had_error = true;
                    continue;
                }
            }
        } else {
            match std::fs::File::open(path).and_then(count_stream) {
                Ok(c) => c,
                Err(err) => {
                    error_path("wc", path, err);
                    had_error = true;
                    continue;
                }
            }
        };
        print_counts(
            &counts,
            show_lines,
            show_words,
            show_bytes,
            path,
            multiple || path != "-",
        );
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

/// Count lines/words/bytes from `reader` in fixed-size chunks,
/// without ever holding the whole input in memory at once.
fn count_stream<R: Read>(mut reader: R) -> io::Result<Counts> {
    let mut buf = [0u8; CHUNK_SIZE];
    let mut counts = Counts::default();
    let mut in_word = false;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        counts.bytes += n as u64;
        for &b in &buf[..n] {
            if b == b'\n' {
                counts.lines += 1;
            }
            let is_whitespace = matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
            if is_whitespace {
                in_word = false;
            } else if !in_word {
                in_word = true;
                counts.words += 1;
            }
        }
    }
    Ok(counts)
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
