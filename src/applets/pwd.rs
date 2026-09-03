//! `pwd` -- print the current working directory.

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "pwd -- print the current working directory";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    let cwd = std::env::current_dir().map_err(|e| AppError::new(e.to_string()))?;
    println!("{}", cwd.display());
    Ok(())
}
