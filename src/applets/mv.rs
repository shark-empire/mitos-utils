//! `mv` -- move/rename files and directories. Falls back to
//! copy-then-remove when the rename crosses filesystems (since
//! `std::fs::rename` fails with EXDEV in that case), preserving each
//! file's modification time in that fallback -- a plain rename
//! never touches mtime, and a cross-device move should look the
//! same to callers either way. Supports `-i`/`--interactive`
//! (confirm before overwriting an existing destination).

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{confirm, error_path};
use crate::common::paths::{walk, walk_post_order};
use std::fs;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "mv [-i] SOURCE... DEST -- move/rename files or directories";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut interactive = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-i" | "--interactive" => interactive = true,
            _ => rest.push(arg),
        }
    }
    rest.extend(forced);
    if rest.len() < 2 {
        return Err(AppError::usage("missing file operand"));
    }

    let dest = PathBuf::from(rest.pop().unwrap());
    let sources = rest;
    let dest_is_dir = dest.is_dir();
    if sources.len() > 1 && !dest_is_dir {
        return Err(AppError::new(format!(
            "target '{}' is not a directory",
            dest.display()
        )));
    }

    let mut had_error = false;
    for src in &sources {
        let src_path = Path::new(src);
        let target = if dest_is_dir {
            dest.join(src_path.file_name().unwrap_or_default())
        } else {
            dest.clone()
        };

        if interactive
            && target.exists()
            && !confirm(&format!("mv: overwrite '{}'?", target.display()))
        {
            continue;
        }

        if let Err(err) = move_one(src_path, &target) {
            error_path("mv", src, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn move_one(src: &Path, dest: &Path) -> std::io::Result<()> {
    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Likely crossed a filesystem boundary: copy then remove.
            if src.is_dir() {
                for entry in walk(src)? {
                    let rel = entry.strip_prefix(src).unwrap_or(&entry);
                    let target = dest.join(rel);
                    if entry.is_dir() {
                        fs::create_dir_all(&target)?;
                    } else {
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::copy(&entry, &target)?;
                        preserve_mtime(&entry, &target)?;
                    }
                }
                preserve_mtime(src, dest)?;
                for entry in walk_post_order(src)? {
                    if entry.is_dir() {
                        let _ = fs::remove_dir(&entry);
                    } else {
                        let _ = fs::remove_file(&entry);
                    }
                }
            } else {
                fs::copy(src, dest)?;
                preserve_mtime(src, dest)?;
                fs::remove_file(src)?;
            }
            Ok(())
        }
    }
}

fn preserve_mtime(src: &Path, dest: &Path) -> std::io::Result<()> {
    let mtime = fs::metadata(src)?.modified()?;
    fs::OpenOptions::new()
        .read(true)
        .open(dest)?
        .set_modified(mtime)
}
