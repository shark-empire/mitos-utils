//! Consistent stderr reporting and small display helpers (byte-size
//! formatting, column layout) shared across utilities.

use std::fmt;
use std::io::{self, BufWriter, Write};

/// Buffer size for `stdout_writer()`. `std::io::Stdout` is internally
/// a `LineWriter`, which scans for (and flushes through) the last
/// newline on every `write()` call it receives -- fine for one big
/// write, but ruinous for callers that write one line at a time
/// (`println!`/`writeln!` in a loop means one `write(2)` syscall per
/// line). Wrapping stdout in this `BufWriter` means that scan-and-
/// flush only happens once per buffer's worth of output instead of
/// once per line, turning "N syscalls" into roughly "N / 16K lines
/// worth of syscalls" for any utility that emits many short lines.
const STDOUT_BUFFER_SIZE: usize = 64 * 1024;

/// A buffered handle to stdout for utilities that write many small
/// records (`cat`, `grep`, `sort`, `uniq`, `cut`, `ls`, `du`, ...).
/// Batches writes into `STDOUT_BUFFER_SIZE`-ish chunks instead of one
/// `write(2)` syscall per line -- see `STDOUT_BUFFER_SIZE`'s doc
/// comment for why that matters given `Stdout`'s own internal
/// line-buffering. Callers must `.flush()` the result before
/// returning `Ok(())` -- a `BufWriter` dropped with unflushed data
/// silently discards any write error (e.g. a downstream pipe that
/// closed early) instead of surfacing it, which is exactly the kind
/// of silent-data-loss bug this crate's `reset_sigpipe()` +
/// `AppError` conventions otherwise take care to avoid.
pub fn stdout_writer() -> BufWriter<io::StdoutLock<'static>> {
    BufWriter::with_capacity(STDOUT_BUFFER_SIZE, io::stdout().lock())
}

/// `"<prog>: <msg>"` on stderr -- the standard coreutils error shape.
pub fn error(prog: &str, msg: impl fmt::Display) {
    eprintln!("{}: {}", prog, msg);
}

/// Reset `SIGPIPE` to its default disposition (terminate the
/// process) on unix. Rust's runtime ignores `SIGPIPE` by default,
/// which turns a closed downstream pipe (e.g. `mitos-cat bigfile |
/// head`) into a `BrokenPipe` `io::Error` that `println!`/`writeln!`
/// then panic on -- not what any POSIX text tool should do. Called
/// once from `errors::run` so every utility that goes through it
/// gets the classic, quiet "killed by SIGPIPE" behavior instead of a
/// panic and a scary backtrace.
#[cfg(unix)]
pub fn reset_sigpipe() {
    mod ffi {
        extern "C" {
            pub fn signal(signum: i32, handler: usize) -> usize;
        }
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        ffi::signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
pub fn reset_sigpipe() {}

/// `"<prog>: <path>: <msg>"` on stderr, for errors tied to one path
/// among possibly several arguments (used heavily by cat/cp/mv/rm/ls
/// and friends when looping over multiple files).
pub fn error_path(prog: &str, path: impl fmt::Display, msg: impl fmt::Display) {
    eprintln!("{}: {}: {}", prog, path, msg);
}

/// Prompt on stderr and read a yes/no answer from stdin -- used by
/// `-i`/`--interactive` on `rm`, `cp`, and `mv`. Answers "no" if
/// stdin can't be read (e.g. running non-interactively with stdin
/// closed or redirected from `/dev/null`), matching coreutils' fail
/// safe rather than fail-destructive behavior in that case.
pub fn confirm(prompt: &str) -> bool {
    eprint!("{} ", prompt);
    let _ = io::stderr().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().chars().next(), Some('y') | Some('Y'))
}

/// Render a byte count the way `ls -h` / `du -h` / `df -h` / `free
/// -h` do: the smallest unit where the value is < 1024, one decimal
/// place unless the whole-unit value is already >= 10.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    if bytes < 1024 {
        return format!("{}{}", bytes, UNITS[0]);
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{:.1}{}", value, UNITS[unit])
    } else {
        format!("{:.0}{}", value, UNITS[unit])
    }
}

/// Lay `names` out in as many columns as fit `term_width`, filling
/// down each column before moving to the next -- the order `ls`
/// uses. Falls back to one name per line when `term_width` is 0
/// (unknown, e.g. output isn't a terminal).
///
/// Writes through the caller-supplied `out` -- typically the
/// caller's own `stdout_writer()` -- rather than managing its own
/// stdout handle, so a caller that's already batching other output
/// into the same buffered writer (e.g. `ls`'s per-target header
/// lines when listing several directories) doesn't need to flush
/// around this call just to keep everything in the right order; the
/// caller flushes once, when it's done. Also reuses one `String`
/// buffer across rows instead of allocating a fresh one each time --
/// for a directory with thousands of entries this is the difference
/// between one `write(2)` call per row and a handful for the whole
/// listing.
pub fn columnate(names: &[String], term_width: usize, out: &mut impl Write) {
    if names.is_empty() {
        return;
    }
    if term_width == 0 {
        for name in names {
            let _ = writeln!(out, "{}", name);
        }
        return;
    }
    let col_width = names.iter().map(|n| n.chars().count()).max().unwrap_or(0) + 2;
    let cols = std::cmp::max(1, term_width / col_width.max(1));
    let rows = (names.len() + cols - 1) / cols;

    let mut line = String::with_capacity(term_width.max(col_width));
    for row in 0..rows {
        line.clear();
        for col in 0..cols {
            let idx = col * rows + row;
            let Some(name) = names.get(idx) else { continue };
            let is_last_in_row = col + 1 == cols || idx + rows >= names.len();
            if is_last_in_row {
                line.push_str(name);
            } else {
                // `fmt::Write` (not `io::Write`, already imported
                // above) for padding into a `String`; `as _` avoids
                // clashing with that existing `Write` import while
                // still bringing `write!`'s target method into scope.
                use std::fmt::Write as _;
                let _ = write!(line, "{:<width$}", name, width = col_width);
            }
        }
        let _ = writeln!(out, "{}", line.trim_end());
    }
}
