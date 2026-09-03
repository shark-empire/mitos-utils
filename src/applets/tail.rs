//! `tail` -- print the last N lines of a file (default 10).
//! Supports `-n N`/`-N` (lines), `-c N` (bytes -- binary-safe, unlike
//! this crate's line-mode text tools; seeks from the end for regular
//! files, buffers for stdin/pipes), and stdin.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};

enum Mode {
    Lines(usize),
    Bytes(usize),
}

pub const USAGE: &str =
    "tail [-n N] [-c N] [FILE...] -- print the last N lines (default 10) or bytes";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut mode = Mode::Lines(10);
    let mut files: Vec<String> = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if arg == "-n" {
            let n = args.next().ok_or_else(|| AppError::usage("option '-n' requires an argument"))?;
            mode = Mode::Lines(n.parse().map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?);
        } else if arg == "-c" {
            let n = args.next().ok_or_else(|| AppError::usage("option '-c' requires an argument"))?;
            mode = Mode::Bytes(n.parse().map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?);
        } else if let Some(n) = arg.strip_prefix('-').filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty()) {
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

        let result = match &mode {
            Mode::Lines(count) => print_last_lines(path, *count),
            Mode::Bytes(count) => print_last_bytes(path, *count),
        };
        if let Err(err) = result {
            error_path("tail", path, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn open(path: &str) -> io::Result<Box<dyn Read>> {
    if path == "-" {
        Ok(Box::new(io::stdin()))
    } else {
        Ok(Box::new(std::fs::File::open(path)?))
    }
}

fn print_last_lines(path: &str, count: usize) -> io::Result<()> {
    let reader = BufReader::new(open(path)?);
    let mut ring: VecDeque<String> = VecDeque::with_capacity(count + 1);
    for line in reader.lines().flatten() {
        if ring.len() == count {
            ring.pop_front();
        }
        ring.push_back(line);
    }
    for line in ring {
        println!("{}", line);
    }
    Ok(())
}

fn print_last_bytes(path: &str, count: usize) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if path != "-" {
        // Regular file: seek from the end instead of buffering the
        // whole thing, so this stays cheap on large files.
        let mut file = std::fs::File::open(path)?;
        let len = file.seek(SeekFrom::End(0))?;
        let start = len.saturating_sub(count as u64);
        file.seek(SeekFrom::Start(start))?;
        io::copy(&mut file, &mut out)?;
        return Ok(());
    }

    // stdin isn't seekable: keep only the last `count` bytes seen.
    let mut ring: VecDeque<u8> = VecDeque::with_capacity(count);
    let mut buf = [0u8; 64 * 1024];
    let mut stdin = io::stdin();
    loop {
        let n = stdin.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            if ring.len() == count {
                ring.pop_front();
            }
            ring.push_back(b);
        }
    }
    let bytes: Vec<u8> = ring.into_iter().collect();
    out.write_all(&bytes)
}
