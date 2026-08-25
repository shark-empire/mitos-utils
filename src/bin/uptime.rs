//! `uptime` -- print how long the system has been running. Reads
//! Linux's `/proc/uptime` when available. mitosOS has no `/proc`
//! yet (see docs/architecture.md); once it grows an equivalent
//! interface, only this file needs to change.

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("uptime", real_main)
}

#[cfg(target_os = "linux")]
fn real_main() -> AppResult<()> {
    let content = std::fs::read_to_string("/proc/uptime")
        .map_err(|e| AppError::new(format!("cannot read /proc/uptime: {}", e)))?;
    let seconds: f64 = content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| AppError::new("unexpected /proc/uptime format"))?;
    println!("up {}", format_duration(seconds as u64));
    Ok(())
}

#[cfg(target_os = "linux")]
fn format_duration(mut secs: u64) -> String {
    let days = secs / 86400;
    secs %= 86400;
    let hours = secs / 3600;
    secs %= 3600;
    let minutes = secs / 60;
    if days > 0 {
        format!("{} days, {:02}:{:02}", days, hours, minutes)
    } else {
        format!("{:02}:{:02}", hours, minutes)
    }
}

#[cfg(not(target_os = "linux"))]
fn real_main() -> AppResult<()> {
    Err(AppError::new(
        "uptime source not available on this target (mitosOS has no /proc yet)",
    ))
}
