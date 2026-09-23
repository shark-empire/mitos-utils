//! `nproc` -- print the number of available processing units.
//!
//! Delegates to `std::thread::available_parallelism` rather than
//! parsing `/sys/devices/system/cpu` by hand -- std already asks the
//! OS correctly (and portably) for this, so there's nothing this
//! module needs to reimplement.

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "nproc -- print the number of available processing units";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    let count = std::thread::available_parallelism()
        .map_err(|e| AppError::new(format!("cannot determine CPU count: {e}")))?;
    println!("{count}");
    Ok(())
}
