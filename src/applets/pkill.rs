//! `pkill` -- send a signal (default `SIGTERM`) to every process
//! whose command name matches a pattern. Reuses `pgrep`'s exact
//! matching logic rather than a second copy of it, and `kill`'s
//! signal-name handling, so all three stay in sync.

use crate::applets::pgrep::matching_processes;
use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str =
    "pkill [-SIGNAL] PATTERN -- signal every process whose command name matches PATTERN";

#[cfg(unix)]
mod ffi {
    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

fn parse_signal(arg: &str, rest: &str) -> AppResult<i32> {
    match rest {
        "9" | "KILL" => Ok(9),
        "15" | "TERM" => Ok(15),
        "1" | "HUP" => Ok(1),
        "2" | "INT" => Ok(2),
        other => other
            .parse()
            .map_err(|_| AppError::usage(format!("unknown signal '{arg}'"))),
    }
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut signal = 15i32; // SIGTERM
    let mut pattern: Option<String> = None;

    for arg in args {
        if let Some(rest) = arg.strip_prefix('-') {
            signal = parse_signal(&arg, rest)?;
        } else if pattern.is_none() {
            pattern = Some(arg);
        } else {
            return Err(AppError::usage(format!("unexpected argument '{arg}'")));
        }
    }
    let pattern = pattern.ok_or_else(|| AppError::usage("missing pattern"))?;

    let targets = matching_processes(&pattern)?;
    if targets.is_empty() {
        return Err(AppError::silent(1));
    }
    send_all(&targets, signal)
}

#[cfg(unix)]
fn send_all(targets: &[(u32, String)], signal: i32) -> AppResult<()> {
    let mut had_error = false;
    for (pid, _comm) in targets {
        if unsafe { ffi::kill(*pid as i32, signal) } != 0 {
            crate::common::output::error(
                "pkill",
                format!("({pid}) - {}", std::io::Error::last_os_error()),
            );
            had_error = true;
        }
    }
    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn send_all(_targets: &[(u32, String)], _signal: i32) -> AppResult<()> {
    Err(AppError::new("signal delivery not available on this target"))
}
