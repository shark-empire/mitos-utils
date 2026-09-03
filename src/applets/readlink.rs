//! `readlink` -- print the target of a symbolic link. With `-f`,
//! resolves the *entire* path to an absolute, symlink-free form
//! (like `realpath`).

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use crate::common::paths::normalize;
use std::path::Path;

pub const USAGE: &str =
    "readlink [-f] PATH... -- print a symlink's target, or its fully resolved path with -f";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut follow = false;
    let mut targets: Vec<String> = Vec::new();
    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-f" | "--canonicalize" => follow = true,
            _ => targets.push(arg),
        }
    }
    targets.extend(forced);
    if targets.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let cwd = std::env::current_dir().map_err(|e| AppError::new(e.to_string()))?;
    let mut had_error = false;

    for target in &targets {
        let path = Path::new(target);
        if follow {
            match path.canonicalize() {
                Ok(resolved) => println!("{}", resolved.display()),
                Err(_) => println!("{}", normalize(path, &cwd).display()),
            }
            continue;
        }
        match std::fs::read_link(path) {
            Ok(dest) => println!("{}", dest.display()),
            Err(err) => {
                error_path("readlink", target, err);
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
