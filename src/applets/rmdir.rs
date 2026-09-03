//! `rmdir` -- remove empty directories.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::fs;

pub const USAGE: &str = "rmdir DIR... -- remove empty directories";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for dir in &args {
        if let Err(err) = fs::remove_dir(dir) {
            error_path("rmdir", dir, format!("failed to remove: {}", err));
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
