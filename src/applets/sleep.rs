//! `sleep` -- pause for a duration. Accepts a plain number of
//! seconds, or a number with an `s`/`m`/`h` suffix.

use crate::common::errors::{AppError, AppResult};
use std::time::Duration;

pub const USAGE: &str = "sleep DURATION -- pause (accepts s/m/h suffixes)";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let arg = args.into_iter().next().ok_or_else(|| AppError::usage("missing operand"))?;

    let (number_part, multiplier) = match arg.chars().last() {
        Some('s') => (&arg[..arg.len() - 1], 1.0),
        Some('m') => (&arg[..arg.len() - 1], 60.0),
        Some('h') => (&arg[..arg.len() - 1], 3600.0),
        _ => (arg.as_str(), 1.0),
    };
    let seconds: f64 = number_part
        .parse()
        .map_err(|_| AppError::usage(format!("invalid time interval: '{}'", arg)))?;

    std::thread::sleep(Duration::from_secs_f64((seconds * multiplier).max(0.0)));
    Ok(())
}
