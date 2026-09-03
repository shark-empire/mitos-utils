//! `whoami` -- print the effective username of the current user.

use crate::common::errors::AppResult;
use crate::common::users::current_identity;

pub const USAGE: &str = "whoami -- print the current username";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    println!("{}", current_identity().user);
    Ok(())
}
