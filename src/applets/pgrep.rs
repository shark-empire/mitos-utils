//! `pgrep` -- list PIDs of processes whose command name matches a
//! pattern. Matches against `/proc/<pid>/comm` (the short command
//! name, not the full command line with arguments) using the same
//! simple `*`/`?` glob `find -name` uses, not full regex -- reusing
//! that exact matcher rather than a second one, so the two can't
//! quietly drift apart on what counts as a match.

use crate::applets::find::glob_match;
use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "pgrep PATTERN -- list PIDs whose command name matches PATTERN";

/// Shared with `pkill`: every (pid, comm) under `/proc` whose `comm`
/// matches `pattern`. Skips this process's own pid, so `pgrep sh` run
/// from inside a shell script doesn't also list that script's own
/// transient invocation of this command.
#[cfg(target_os = "linux")]
pub(crate) fn matching_processes(pattern: &str) -> AppResult<Vec<(u32, String)>> {
    let my_pid = std::process::id();
    let entries =
        std::fs::read_dir("/proc").map_err(|e| AppError::new(format!("cannot read /proc: {e}")))?;
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == my_pid {
            continue;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if !comm.is_empty() && glob_match(pattern, &comm) {
            matches.push((pid, comm));
        }
    }
    Ok(matches)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn matching_processes(_pattern: &str) -> AppResult<Vec<(u32, String)>> {
    Ok(Vec::new())
}

#[cfg(target_os = "linux")]
pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);
    let pattern = parts
        .into_iter()
        .next()
        .ok_or_else(|| AppError::usage("missing pattern"))?;

    let matches = matching_processes(&pattern)?;
    for (pid, comm) in &matches {
        println!("{pid} {comm}");
    }
    if matches.is_empty() {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::new(
        "process list not available on this target (mitosOS has no /proc yet)",
    ))
}
