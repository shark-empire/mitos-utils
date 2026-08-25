//! `kill` -- send a signal to a process (default `SIGTERM`).
//! Supports `-9`/`-KILL`, `-15`/`-TERM`, `-1`/`-HUP`, `-2`/`-INT`,
//! and a bare `-<n>` for an arbitrary signal number.

use mitos_utils::common::errors::{run, AppError, AppResult};

#[cfg(unix)]
mod ffi {
    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

fn main() -> std::process::ExitCode {
    run("kill", real_main)
}

fn real_main() -> AppResult<()> {
    let mut signal = 15i32; // SIGTERM
    let mut pids: Vec<i32> = Vec::new();

    for arg in std::env::args().skip(1) {
        if let Some(rest) = arg.strip_prefix('-') {
            signal = match rest {
                "9" | "KILL" => 9,
                "15" | "TERM" => 15,
                "1" | "HUP" => 1,
                "2" | "INT" => 2,
                other => other
                    .parse()
                    .map_err(|_| AppError::usage(format!("unknown signal '{}'", arg)))?,
            };
        } else {
            pids.push(
                arg.parse()
                    .map_err(|_| AppError::usage(format!("invalid pid: '{}'", arg)))?,
            );
        }
    }
    if pids.is_empty() {
        return Err(AppError::usage("usage: kill [-SIGNAL] PID..."));
    }

    send_all(&pids, signal)
}

#[cfg(unix)]
fn send_all(pids: &[i32], signal: i32) -> AppResult<()> {
    let mut had_error = false;
    for &pid in pids {
        if unsafe { ffi::kill(pid, signal) } != 0 {
            mitos_utils::common::output::error("kill", format!("({}) - No such process", pid));
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
fn send_all(_pids: &[i32], _signal: i32) -> AppResult<()> {
    Err(AppError::new("signal delivery not available on this target"))
}
