//! `touch` -- create empty files if missing, or update their
//! modified time if they exist.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
use std::fs::{self, OpenOptions};
use std::time::SystemTime;

pub const USAGE: &str = "touch FILE... -- create files / update modification time";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return Err(AppError::usage("missing file operand"));
    }

    let mut had_error = false;
    for path in &args {
        if let Err(err) = touch_one(path) {
            error_path("touch", path, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn touch_one(path: &str) -> std::io::Result<()> {
    if fs::metadata(path).is_ok() {
        let now = SystemTime::now();
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_modified(now)
    } else {
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .map(|_| ())
    }
}
