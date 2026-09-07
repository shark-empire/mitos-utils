//! `cat` -- concatenate files to stdout. Supports `-n`/`--number`,
//! `-b`/`--number-nonblank`, reading multiple files, and `-` for
//! stdin.
//!
//! Binary-safe: everything here operates on raw bytes, never `str`.
//! With neither `-n` nor `-b` (the common case), each file is one
//! buffered byte-for-byte copy straight to stdout -- no line
//! splitting, no UTF-8 validation, no per-line allocation at all.
//! `-n`/`-b` still need line boundaries (to know where to print a
//! number), but find them by scanning for the `\n` byte directly
//! rather than by decoding UTF-8 `String`s, so invalid UTF-8 and NUL
//! bytes pass through unchanged instead of aborting with an error --
//! this used to be a documented limitation (see
//! docs/compatibility.md's binary-safety note) rather than the
//! deliberate, buffered `.lines()`-based design it looked like.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, stdout_writer};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

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

    let numbering = number_all || number_nonblank;
    let mut out = stdout_writer();
    let mut line_no: u64 = 1;
    let mut had_error = false;

    for path in &files {
        let mut opened: Box<dyn Read> = if path == "-" {
            Box::new(io::stdin())
        } else {
            match File::open(path) {
                Ok(f) => Box::new(f),
                Err(err) => {
                    error_path("cat", path, err);
                    had_error = true;
                    continue;
                }
            }
        };

        let result = if numbering {
            number_stream(&mut *opened, &mut out, number_nonblank, &mut line_no)
        } else {
            // Fast path: no per-line work needed at all, so copy raw
            // bytes straight through in large buffered chunks --
            // this is what real `cat` does in the common no-flags
            // case, and it's both faster and binary-safe.
            io::copy(&mut opened, &mut out).map(|_| ())
        };
        if let Err(err) = result {
            error_path("cat", path, err);
            had_error = true;
        }
    }

    let _ = out.flush();
    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

/// Copy `reader` to `out`, prefixing each line with an incrementing
/// number (`-n`), or the same but skipping blank lines (`-b`). Line
/// boundaries are found by scanning raw bytes for `\n`, so this
/// never requires the input to be valid UTF-8.
fn number_stream(
    reader: &mut dyn Read,
    out: &mut impl Write,
    nonblank_only: bool,
    line_no: &mut u64,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut buf: Vec<u8> = Vec::with_capacity(256);
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break; // EOF
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        if nonblank_only && buf.is_empty() {
            out.write_all(b"\n")?;
        } else {
            write!(out, "{:>6}\t", line_no)?;
            out.write_all(&buf)?;
            out.write_all(b"\n")?;
            *line_no += 1;
        }
    }
    Ok(())
}
