//! `rm` -- remove files. Supports `-r`/`-R`/`--recursive` (remove
//! directory trees) and `-f`/`--force` (ignore missing files, never
//! prompt).

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::walk_post_order;
use std::fs;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("rm", real_main)
}

fn real_main() -> AppResult<()> {
    let mut recursive = false;
    let mut force = false;
    let mut targets: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-r" | "-R" | "--recursive" => recursive = true,
            "-f" | "--force" => force = true,
            "-rf" | "-fr" => {
                recursive = true;
                force = true;
            }
            _ => targets.push(arg),
        }
    }
    if targets.is_empty() {
        if force {
            return Ok(());
        }
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for target in &targets {
        let path = Path::new(target);
        if let Err(err) = remove_one(path, recursive) {
            if force && err.kind() == std::io::ErrorKind::NotFound {
                continue;
            }
            error_path("rm", target, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn remove_one(path: &Path, recursive: bool) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.is_symlink() {
        if !recursive {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "is a directory (use -r to remove directories)",
            ));
        }
        for entry in walk_post_order(path)? {
            let entry_meta = fs::symlink_metadata(&entry)?;
            if entry_meta.is_dir() && !entry_meta.is_symlink() {
                fs::remove_dir(&entry)?;
            } else {
                fs::remove_file(&entry)?;
            }
        }
        Ok(())
    } else {
        fs::remove_file(path)
    }
}
