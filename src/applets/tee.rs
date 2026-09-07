//! `tee` -- copy stdin to stdout and to one or more files. Supports
//! `-a`/`--append`.
//!
//! Binary-safe: copies raw bytes in large chunks instead of decoding
//! stdin as UTF-8 lines and re-emitting them with `writeln!` one at
//! a time -- for input with many short lines, that was one
//! `write(2)` syscall *per line, per destination* (stdout and every
//! output file). Reading in chunks and writing each chunk straight
//! through means the syscall count now tracks how much data arrives
//! per `read(2)`, not how many newlines are in it. stdout is flushed
//! after every chunk so interactive use (watching output live while
//! it's also being logged to a file) doesn't get stuck waiting for a
//! full buffer -- `std::fs::File` has no userspace buffering to
//! begin with, so the output files are already written immediately.

use crate::common::errors::{AppError, AppResult};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

pub const USAGE: &str = "tee [-a] FILE... -- copy stdin to stdout and to files";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut append = false;
    let mut files: Vec<String> = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-a" | "--append" => append = true,
            _ => files.push(arg),
        }
    }

    let mut outputs: Vec<std::fs::File> = Vec::new();
    for path in &files {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(path)
            .map_err(|e| AppError::new(format!("{}: {}", path, e)))?;
        outputs.push(file);
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut stdin = io::stdin();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = stdin.read(&mut buf).map_err(AppError::from)?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        let _ = out.write_all(chunk);
        let _ = out.flush();
        for f in &mut outputs {
            let _ = f.write_all(chunk);
        }
    }
    Ok(())
}
