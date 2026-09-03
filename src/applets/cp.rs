//! `cp` -- copy files, or directory trees with `-r`/`-R`/`--recursive`.
//! `-i`/`--interactive` confirms before overwriting an existing
//! destination; `-p`/`--preserve` additionally copies the source's
//! modification time onto the copy (permission bits are already
//! preserved unconditionally -- that's `std::fs::copy`'s own
//! behavior, matching `cp`'s non-`-p` default).

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{confirm, error_path};
use crate::common::paths::walk;
use std::fs;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "cp [-r] [-i] [-p] SOURCE... DEST -- copy files or directory trees";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut recursive = false;
    let mut interactive = false;
    let mut preserve = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-r" | "-R" | "--recursive" => recursive = true,
            "-i" | "--interactive" => interactive = true,
            "-p" | "--preserve" => preserve = true,
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

        if interactive && target.exists() && !confirm(&format!("cp: overwrite '{}'?", target.display())) {
            continue;
        }

        let result = if recursive && src_path.is_dir() {
            copy_tree(src_path, &target, preserve)
        } else {
            copy_file(src_path, &target, preserve)
        };
        if let Err(err) = result {
            error_path("cp", src, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn copy_file(src: &Path, dest: &Path, preserve: bool) -> std::io::Result<()> {
    fs::copy(src, dest)?;
    if preserve {
        preserve_mtime(src, dest)?;
    }
    Ok(())
}

fn copy_tree(src: &Path, dest: &Path, preserve: bool) -> std::io::Result<()> {
    // Per-file mtimes are preserved as each file is copied; the
    // root's own mtime is set once at the end (after every child has
    // been created, since creating a child bumps its parent's
    // mtime). Nested subdirectories in between don't get their own
    // mtime restored -- a small gap versus real `cp -rp`, left as a
    // known limitation (docs/compatibility.md).
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
            if preserve {
                preserve_mtime(&entry, &target)?;
            }
        }
    }
    if preserve {
        preserve_mtime(src, dest)?;
    }
    Ok(())
}

fn preserve_mtime(src: &Path, dest: &Path) -> std::io::Result<()> {
    let mtime = fs::metadata(src)?.modified()?;
    fs::OpenOptions::new().read(true).open(dest)?.set_modified(mtime)
}
