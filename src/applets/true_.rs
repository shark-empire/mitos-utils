//! `true` -- do nothing, successfully. (The module is named
//! `true_` -- `true` is a reserved keyword in Rust and can't be used
//! as a module identifier -- but the applet's registered *name* and
//! the binary it produces are both still plain `true`.)

use crate::common::errors::AppResult;

pub const USAGE: &str = "true -- exit successfully, doing nothing";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    Ok(())
}
