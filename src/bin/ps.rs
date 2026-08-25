//! `ps` -- list running processes. Reads Linux's `/proc/<pid>/`
//! entries when available. mitosOS has no `/proc` yet (see
//! docs/architecture.md) -- this is written against the interface a
//! future mitosOS procfs would need to expose (per-pid `comm` and
//! `stat`), so porting later is a matter of pointing these reads
//! somewhere else.

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("ps", real_main)
}

#[cfg(target_os = "linux")]
fn real_main() -> AppResult<()> {
    println!("{:>7} {:<8} COMMAND", "PID", "STATE");
    let entries =
        std::fs::read_dir("/proc").map_err(|e| AppError::new(format!("cannot read /proc: {}", e)))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let comm = std::fs::read_to_string(format!("/proc/{}/comm", pid))
            .unwrap_or_default()
            .trim()
            .to_string();
        if comm.is_empty() {
            continue;
        }
        let state = std::fs::read_to_string(format!("/proc/{}/stat", pid))
            .ok()
            .and_then(|s| {
                s.split(')')
                    .nth(1)
                    .and_then(|rest| rest.trim().split(' ').next().map(String::from))
            })
            .unwrap_or_else(|| "?".to_string());
        println!("{:>7} {:<8} {}", pid, state, comm);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn real_main() -> AppResult<()> {
    Err(AppError::new(
        "process list not available on this target (mitosOS has no /proc yet)",
    ))
}
