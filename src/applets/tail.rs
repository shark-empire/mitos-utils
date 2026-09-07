//! `tail` -- print the last N lines of a file (default 10).
//! Supports `-n N`/`-N` (lines), `-c N` (bytes), and stdin.
//!
//! For regular, seekable files, both `-n` and `-c` seek backward
//! from the end in chunks instead of reading the whole file --
//! `tail -n 10` on a multi-gigabyte log only ever reads the last
//! chunk or two off disk, not the entire file. Unseekable sources
//! (stdin, pipes, or a path that turns out not to support seeking)
//! fall back to a bounded ring buffer of the last N lines/bytes seen,
//! since there's no way to know where "near the end" is without
//! reading everything up to it exactly once.
//!
//! Byte-oriented throughout (splits only on the `\n` byte, never
//! requires valid UTF-8), so this passes binary data and any line
//! ending through unchanged instead of silently dropping lines that
//! aren't valid UTF-8 the way a `.lines()`-based version would.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, stdout_writer};
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
            let n = args
                .next()
                .ok_or_else(|| AppError::usage("option '-n' requires an argument"))?;
            mode = Mode::Lines(
                n.parse()
                    .map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?,
            );
        } else if arg == "-c" {
            let n = args
                .next()
                .ok_or_else(|| AppError::usage("option '-c' requires an argument"))?;
            mode = Mode::Bytes(
                n.parse()
                    .map_err(|_| AppError::usage(format!("invalid number: '{}'", n)))?,
            );
        } else if let Some(n) = arg
            .strip_prefix('-')
            .filter(|s| s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty())
        {
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
    let mut out = stdout_writer();
    for (i, path) in files.iter().enumerate() {
        if multiple {
            if i > 0 {
                let _ = writeln!(out);
            }
            let _ = writeln!(out, "==> {} <==", path);
        }

        let result = match &mode {
            Mode::Lines(count) => print_last_lines(path, *count, &mut out),
            Mode::Bytes(count) => print_last_bytes(path, *count, &mut out),
        };
        if let Err(err) = result {
            error_path("tail", path, err);
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

const CHUNK_SIZE: u64 = 64 * 1024;

fn print_last_lines(path: &str, count: usize, out: &mut impl Write) -> io::Result<()> {
    if path != "-" {
        let mut file = std::fs::File::open(path)?;
        if file.seek(SeekFrom::End(0)).is_ok() {
            let start = last_n_lines_offset(&mut file, count)?;
            file.seek(SeekFrom::Start(start))?;
            io::copy(&mut file, out)?;
            return Ok(());
        }
        // Opened fine but isn't seekable (e.g. a FIFO given by
        // path) -- fall back to the same bounded streaming approach
        // used for stdin below.
        return stream_last_lines(file, count, out);
    }
    stream_last_lines(io::stdin(), count, out)
}

/// Find the byte offset in `file` where the last `count` lines
/// begin, by scanning backward in `CHUNK_SIZE` chunks and counting
/// `\n` bytes -- without ever reading more of the file than the
/// tail end actually asked for. A `\n` that is the very last byte of
/// the file terminates the final line rather than starting a new
/// (empty) one, so it isn't counted as one of the `count` separators
/// being searched for. Returns `0` (start of file) if the file has
/// fewer than `count` lines.
fn last_n_lines_offset(file: &mut std::fs::File, count: usize) -> io::Result<u64> {
    let file_len = file.seek(SeekFrom::End(0))?;
    if count == 0 || file_len == 0 {
        return Ok(file_len);
    }

    let mut pos = file_len;
    let mut newlines_seen: usize = 0;
    let mut buf = vec![0u8; CHUNK_SIZE as usize];

    while pos > 0 {
        let read_len = std::cmp::min(CHUNK_SIZE, pos) as usize;
        pos -= read_len as u64;
        file.seek(SeekFrom::Start(pos))?;
        file.read_exact(&mut buf[..read_len])?;

        let mut i = read_len;
        while i > 0 {
            i -= 1;
            let abs = pos + i as u64;
            if buf[i] == b'\n' && abs != file_len - 1 {
                newlines_seen += 1;
                if newlines_seen == count {
                    return Ok(abs + 1);
                }
            }
        }
    }
    Ok(0)
}

/// Bounded-memory fallback for sources that can't be seeked: keep
/// only the last `count` lines seen, streaming the rest through
/// without holding it all in memory.
fn stream_last_lines(source: impl Read, count: usize, out: &mut impl Write) -> io::Result<()> {
    let mut reader = BufReader::new(source);
    let mut ring: VecDeque<Vec<u8>> = VecDeque::with_capacity(count.min(8192) + 1);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        ring.push_back(buf.clone());
        while ring.len() > count {
            ring.pop_front();
        }
    }
    for line in &ring {
        out.write_all(line)?;
    }
    Ok(())
}

fn print_last_bytes(path: &str, count: usize, out: &mut impl Write) -> io::Result<()> {
    if path != "-" {
        // Regular file: seek from the end instead of buffering the
        // whole thing, so this stays cheap on large files.
        let mut file = std::fs::File::open(path)?;
        if let Ok(len) = file.seek(SeekFrom::End(0)) {
            let start = len.saturating_sub(count as u64);
            file.seek(SeekFrom::Start(start))?;
            io::copy(&mut file, out)?;
            return Ok(());
        }
        return stream_last_bytes(file, count, out);
    }
    stream_last_bytes(io::stdin(), count, out)
}

/// Bounded-memory fallback for unseekable byte sources: keep only
/// the last `count` bytes seen.
fn stream_last_bytes(mut source: impl Read, count: usize, out: &mut impl Write) -> io::Result<()> {
    let mut ring: VecDeque<u8> = VecDeque::with_capacity(count.min(1 << 20));
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = source.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            ring.push_back(b);
            if ring.len() > count {
                ring.pop_front();
            }
        }
    }
    let bytes: Vec<u8> = ring.into_iter().collect();
    out.write_all(&bytes)
}
