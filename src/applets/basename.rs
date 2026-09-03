//! `basename` -- strip directory and (optionally) a trailing suffix
//! from a path.

use crate::common::errors::{AppError, AppResult};
use crate::common::paths::basename;

pub const USAGE: &str = "basename PATH [SUFFIX] -- strip directory (and suffix) from a path";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut name = basename(&args[0]);
    if let Some(suffix) = args.get(1) {
        if !suffix.is_empty() && name != *suffix {
            if let Some(stripped) = name.strip_suffix(suffix.as_str()) {
                name = stripped.to_string();
            }
        }
    }
    println!("{}", name);
    Ok(())
}
