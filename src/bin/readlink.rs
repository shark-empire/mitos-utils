//! `readlink` -- print the target of a symbolic link. With `-f`,
//! resolves the *entire* path to an absolute, symlink-free form
//! (like `realpath`).

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::normalize;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("readlink", real_main)
}

fn real_main() -> AppResult<()> {
    let mut follow = false;
    let mut targets: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-f" | "--canonicalize" => follow = true,
            _ => targets.push(arg),
        }
    }
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
