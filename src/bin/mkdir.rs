//! `mkdir` -- create directories. Supports `-p`/`--parents`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use std::fs;

fn main() -> std::process::ExitCode {
    run("mkdir", real_main)
}

fn real_main() -> AppResult<()> {
    let mut parents = false;
    let mut dirs: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-p" | "--parents" => parents = true,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                return Err(AppError::usage(format!("unrecognized option '{}'", arg)))
            }
            _ => dirs.push(arg),
        }
    }
    if dirs.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for dir in &dirs {
        let result = if parents { fs::create_dir_all(dir) } else { fs::create_dir(dir) };
        if let Err(err) = result {
            error_path("mkdir", dir, format!("cannot create directory: {}", err));
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
