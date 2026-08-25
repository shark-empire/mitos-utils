//! `pwd` -- print the current working directory.

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("pwd", real_main)
}

fn real_main() -> AppResult<()> {
    let cwd = std::env::current_dir().map_err(|e| AppError::new(e.to_string()))?;
    println!("{}", cwd.display());
    Ok(())
}
