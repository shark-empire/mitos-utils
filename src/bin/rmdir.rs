//! `rmdir` -- remove empty directories.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use std::fs;

fn main() -> std::process::ExitCode {
    run("rmdir", real_main)
}

fn real_main() -> AppResult<()> {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut had_error = false;
    for dir in &dirs {
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
