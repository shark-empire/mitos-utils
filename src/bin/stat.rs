//! `stat` -- display detailed file status: type, size, mode,
//! owner/group, and modification time.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
#[cfg(unix)]
use mitos_utils::common::permissions::{file_type_char, format_mode, mode_of};
use std::time::UNIX_EPOCH;

fn main() -> std::process::ExitCode {
    run("stat", real_main)
}

fn real_main() -> AppResult<()> {
    let targets: Vec<String> = std::env::args().skip(1).collect();
    if targets.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for target in &targets {
        match std::fs::symlink_metadata(target) {
            Ok(meta) => print_stat(target, &meta),
            Err(err) => {
                error_path("stat", target, err);
                had_error = true;
            }
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn print_stat(name: &str, meta: &std::fs::Metadata) {
    use std::os::unix::fs::MetadataExt;
    let mode = mode_of(meta);
    let type_char = file_type_char(meta);
    let kind = match type_char {
        'd' => "directory",
        'l' => "symbolic link",
        'b' => "block device",
        'c' => "character device",
        'p' => "FIFO",
        's' => "socket",
        _ => "regular file",
    };
    let owner = mitos_utils::common::users::name_for_uid(meta.uid())
        .unwrap_or_else(|| meta.uid().to_string());
    let group = mitos_utils::common::users::name_for_gid(meta.gid())
        .unwrap_or_else(|| meta.gid().to_string());
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    println!("  File: {}", name);
    println!("  Size: {:<12} Blocks: {:<10} {}", meta.len(), meta.blocks(), kind);
    println!(
        "Access: ({:04o}/{})  Uid: ( {:>5}/{:>8})  Gid: ( {:>5}/{:>8})",
        mode,
        format_mode(mode, type_char),
        meta.uid(),
        owner,
        meta.gid(),
        group
    );
    println!("Modify: {} (unix epoch seconds)", mtime);
}

#[cfg(not(unix))]
fn print_stat(name: &str, meta: &std::fs::Metadata) {
    println!("  File: {}", name);
    println!("  Size: {}", meta.len());
}
