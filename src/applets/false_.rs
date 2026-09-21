//! `false` -- do nothing, unsuccessfully. The mirror image of `true`
//! (see that module's doc comment for why the module is named
//! `true_` -- `true`/`false` are both reserved keywords in Rust, so
//! neither can be used as a module identifier; the registered applet
//! *name* and the binary this produces are still plain `false`).

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "false -- exit unsuccessfully, doing nothing";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::silent(1))
}
