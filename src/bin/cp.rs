//! `cp` -- copy files, or directory trees with `-r`/`-R`/`--recursive`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::walk;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> std::process::ExitCode {
    run("cp", real_main)
}

fn real_main() -> AppResult<()> {
    let mut recursive = false;
    let mut rest: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-r" | "-R" | "--recursive" => recursive = true,
            _ => rest.push(arg),
        }
    }
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
        let result = if recursive && src_path.is_dir() {
            copy_tree(src_path, &target)
        } else {
            fs::copy(src_path, &target).map(|_| ())
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

fn copy_tree(src: &Path, dest: &Path) -> std::io::Result<()> {
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
    Ok(())
}
