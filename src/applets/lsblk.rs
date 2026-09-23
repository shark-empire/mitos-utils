//! `lsblk` -- list block devices and their partitions, reading
//! `/sys/block` (Linux). A summary, not a full replica of the real
//! `lsblk` -- filesystem type/UUID/mountpoint (which real `lsblk`
//! gets via `libblkid`) aren't covered here, since parsing every
//! filesystem's own superblock format to identify it is a much
//! bigger undertaking than reading a few plain sysfs attribute files.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::human_size;

pub const USAGE: &str = "lsblk -- list block devices and their partitions";

#[cfg(target_os = "linux")]
fn read_u64(path: &str) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(target_os = "linux")]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    let entries = std::fs::read_dir("/sys/block")
        .map_err(|e| AppError::new(format!("cannot read /sys/block: {e}")))?;

    let mut devices: Vec<_> = entries.flatten().collect();
    devices.sort_by_key(|e| e.file_name());

    println!("{:<10} {:>8} {:<3} {}", "NAME", "SIZE", "RO", "TYPE");
    for entry in devices {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let base = format!("/sys/block/{name}");

        // /sys/block/<dev>/size is always in 512-byte sectors,
        // regardless of the device's real logical block size.
        let bytes = read_u64(&format!("{base}/size")) * 512;
        let ro = read_u64(&format!("{base}/ro"));
        let device_type = if std::path::Path::new(&format!("{base}/device")).exists() {
            "disk"
        } else {
            "loop/virtual"
        };
        println!(
            "{:<10} {:>8} {:<3} {}",
            name,
            human_size(bytes),
            ro,
            device_type
        );

        let Ok(subentries) = std::fs::read_dir(&base) else {
            continue;
        };
        let mut partitions: Vec<_> = subentries
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.starts_with(&name) && s != name)
                    .unwrap_or(false)
            })
            .collect();
        partitions.sort_by_key(|e| e.file_name());
        for part in partitions {
            let Some(pname) = part.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let psize = read_u64(&format!("{base}/{pname}/size")) * 512;
            println!(
                "  {:<8} {:>8} {:<3} {}",
                pname,
                human_size(psize),
                "-",
                "part"
            );
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::new(
        "block device list not available on this target (needs /sys/block)",
    ))
}
