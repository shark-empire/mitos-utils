//! `which` -- locate a command by searching `$PATH`.
//!
//! Silent (no stdout line, just a nonzero exit) for a command that
//! isn't found, matching `command -v`'s behavior rather than the
//! "no `cmd` in ($PATH)" some `which` implementations print --
//! keeps output script-friendly by default, matching this crate's
//! general "machine-readable output" design goal.

use crate::common::errors::{AppError, AppResult};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "which COMMAND... -- print the full path of each command found on $PATH";

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// A name containing a `/` is already a path, not a bare command
/// name -- checked directly instead of searched for on `$PATH`,
/// matching how a shell itself resolves it.
fn find_on_path(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let path = PathBuf::from(name);
        return is_executable(&path).then_some(path);
    }
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut names = opts;
    names.extend(forced);

    if names.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut all_found = true;
    for name in &names {
        match find_on_path(name) {
            Some(path) => println!("{}", path.display()),
            None => all_found = false,
        }
    }

    if all_found {
        Ok(())
    } else {
        Err(AppError::silent(1))
    }
}
