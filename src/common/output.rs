//! Consistent stderr reporting and small display helpers (byte-size
//! formatting, column layout) shared across utilities.

use std::fmt;
use std::io;

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
    use std::io::Write;
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
pub fn columnate(names: &[String], term_width: usize) {
    if names.is_empty() {
        return;
    }
    if term_width == 0 {
        for name in names {
            println!("{}", name);
        }
        return;
    }
    let col_width = names.iter().map(|n| n.chars().count()).max().unwrap_or(0) + 2;
    let cols = std::cmp::max(1, term_width / col_width.max(1));
    let rows = (names.len() + cols - 1) / cols;

    for row in 0..rows {
        let mut line = String::new();
        for col in 0..cols {
            let idx = col * rows + row;
            let Some(name) = names.get(idx) else { continue };
            let is_last_in_row = col + 1 == cols || idx + rows >= names.len();
            if is_last_in_row {
                line.push_str(name);
            } else {
                line.push_str(&format!("{:<width$}", name, width = col_width));
            }
        }
        println!("{}", line.trim_end());
    }
}
