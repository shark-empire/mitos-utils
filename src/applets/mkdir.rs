//! `mkdir` -- create directories. Supports `-p`/`--parents`.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::fs;

pub const USAGE: &str = "mkdir [-p] DIR... -- create directories";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut parents = false;
    let mut dirs: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-p" | "--parents" => parents = true,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                return Err(AppError::usage(format!("unrecognized option '{}'", arg)))
            }
            _ => dirs.push(arg),
        }
    }
    dirs.extend(forced);
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
