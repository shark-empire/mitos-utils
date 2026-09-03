//! `rm` -- remove files. Supports `-r`/`-R`/`--recursive` (remove
//! directory trees), `-f`/`--force` (ignore missing files, never
//! prompt), and `-i`/`--interactive` (confirm before every removal;
//! `-f` given after `-i` wins, matching GNU's last-flag-wins rule
//! for this pair). Recursive removal on Linux goes through
//! `common::safewalk` for TOCTOU-safe deletion (see that module's
//! docs); other targets use the plain path-based walk.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{confirm, error_path};
#[cfg(not(target_os = "linux"))]
use crate::common::paths::walk_post_order;
use std::fs;
use std::path::Path;

pub const USAGE: &str = "rm [-r] [-f] [-i] FILE... -- remove files or directory trees";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut recursive = false;
    let mut force = false;
    let mut interactive = false;
    let mut targets: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-r" | "-R" | "--recursive" => recursive = true,
            "-f" | "--force" => {
                force = true;
                interactive = false;
            }
            "-i" | "--interactive" => {
                interactive = true;
                force = false;
            }
            "-rf" | "-fr" => {
                recursive = true;
                force = true;
                interactive = false;
            }
            "-ri" | "-ir" => {
                recursive = true;
                interactive = true;
                force = false;
            }
            _ => targets.push(arg),
        }
    }
    targets.extend(forced);
    if targets.is_empty() {
        if force {
            return Ok(());
        }
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for target in &targets {
        if interactive && !confirm(&format!("rm: remove '{}'?", target)) {
            continue;
        }
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

#[cfg(target_os = "linux")]
fn remove_one(path: &Path, recursive: bool) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.is_symlink() {
        if !recursive {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "is a directory (use -r to remove directories)",
            ));
        }
        crate::common::safewalk::remove_tree(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(not(target_os = "linux"))]
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
