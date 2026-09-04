//! `chmod` -- change file mode bits. Accepts octal (`755`) or
//! symbolic (`u+x`, `go-w`) mode specs (see
//! `common::permissions::parse_mode`), and `-R`/`--recursive`.
//! Recursive mode on Linux goes through `common::safewalk` for
//! TOCTOU-safe traversal (see that module's docs); other targets use
//! the plain path-based walk. Either way, a symbolic spec like
//! `u+x` is computed relative to each entry's own current mode, not
//! one flat value copied onto everything in the tree.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
#[cfg(not(target_os = "linux"))]
use crate::common::paths::walk;
use crate::common::permissions::parse_mode;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub const USAGE: &str = "chmod [-R] MODE FILE... -- change file mode (octal or symbolic)";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut recursive = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-R" | "--recursive" => recursive = true,
            _ => rest.push(arg),
        }
    }
    rest.extend(forced);
    if rest.len() < 2 {
        return Err(AppError::usage("usage: chmod [-R] MODE FILE..."));
    }
    let spec = rest.remove(0);
    let files = rest;

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        if let Err(err) = chmod_one(path, &spec, recursive) {
            error_path("chmod", file, err);
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
fn chmod_one(path: &Path, spec: &str, recursive: bool) -> std::io::Result<()> {
    // Validate the spec once up front (parse_mode's success/failure
    // depends only on the spec's syntax, never on the current-mode
    // value passed in). `safewalk::chmod_tree` takes an infallible
    // `Fn(u32) -> u32` -- it has no way to propagate a per-entry
    // parse error out of its closure -- so a malformed spec needs to
    // be caught here, clearly, rather than silently falling back to
    // "leave the mode unchanged" inside the closure for every entry
    // in the tree.
    parse_mode(spec, 0).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let spec_owned = spec.to_string();
    if recursive && path.is_dir() && !path.is_symlink() {
        return crate::common::safewalk::chmod_tree(path, &move |current| {
            parse_mode(&spec_owned, current).unwrap_or(current)
        });
    }
    let current = fs::metadata(path)?.permissions().mode() & 0o7777;
    let new_mode = parse_mode(spec, current)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    fs::set_permissions(path, fs::Permissions::from_mode(new_mode))
}

#[cfg(not(target_os = "linux"))]
fn chmod_one(path: &Path, spec: &str, recursive: bool) -> std::io::Result<()> {
    let targets = if recursive {
        walk(path)?
    } else {
        vec![path.to_path_buf()]
    };
    for target in &targets {
        let current = fs::metadata(target)?.permissions().mode() & 0o7777;
        let new_mode = parse_mode(spec, current)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        fs::set_permissions(target, fs::Permissions::from_mode(new_mode))?;
    }
    Ok(())
}
