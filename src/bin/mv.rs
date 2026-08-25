//! `mv` -- move/rename files and directories. Falls back to
//! copy-then-remove when the rename crosses filesystems (matching
//! GNU `mv`'s behavior, since `std::fs::rename` fails with EXDEV in
//! that case).

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::{walk, walk_post_order};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> std::process::ExitCode {
    run("mv", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err(AppError::usage("missing file operand"));
    }

    let mut args = args;
    let dest = PathBuf::from(args.pop().unwrap());
    let sources = args;
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
                    }
                }
                for entry in walk_post_order(src)? {
                    if entry.is_dir() {
                        let _ = fs::remove_dir(&entry);
                    } else {
                        let _ = fs::remove_file(&entry);
                    }
                }
            } else {
                fs::copy(src, dest)?;
                fs::remove_file(src)?;
            }
            Ok(())
        }
    }
}
