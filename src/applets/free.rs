//! `free` -- display memory usage. Reads Linux's `/proc/meminfo`
//! when available. mitosOS has no `/proc` yet (see
//! docs/architecture.md).

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "free -- print memory usage (Linux)";

#[cfg(target_os = "linux")]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    use crate::common::output::human_size;
    use std::collections::HashMap;

    let content = std::fs::read_to_string("/proc/meminfo")
        .map_err(|e| AppError::new(format!("cannot read /proc/meminfo: {}", e)))?;
    let mut values: HashMap<String, u64> = HashMap::new();
    for line in content.lines() {
        if let Some((key, rest)) = line.split_once(':') {
            let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse().unwrap_or(0);
            values.insert(key.trim().to_string(), kb * 1024);
        }
    }
    let total = values.get("MemTotal").copied().unwrap_or(0);
    let free = values.get("MemFree").copied().unwrap_or(0);
    let available = values.get("MemAvailable").copied().unwrap_or(free);
    let used = total.saturating_sub(free);
    let swap_total = values.get("SwapTotal").copied().unwrap_or(0);
    let swap_free = values.get("SwapFree").copied().unwrap_or(0);
    let swap_used = swap_total.saturating_sub(swap_free);

    println!("{:>12} {:>10} {:>10} {:>10} {:>10}", "", "total", "used", "free", "available");
    println!(
        "{:<12} {:>10} {:>10} {:>10} {:>10}",
        "Mem:", human_size(total), human_size(used), human_size(free), human_size(available)
    );
    println!(
        "{:<12} {:>10} {:>10} {:>10}",
        "Swap:", human_size(swap_total), human_size(swap_used), human_size(swap_free)
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::new(
        "memory info not available on this target (mitosOS has no /proc yet)",
    ))
}
