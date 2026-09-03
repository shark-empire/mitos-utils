//! `dirname` -- strip the last path component, printing what's left.

use crate::common::errors::{AppError, AppResult};
use crate::common::paths::dirname;

pub const USAGE: &str = "dirname PATH... -- strip the last component from a path";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }
    for path in &args {
        println!("{}", dirname(path));
    }
    Ok(())
}
