//! `whoami` -- print the effective username of the current user.

use mitos_utils::common::errors::{run, AppResult};
use mitos_utils::common::users::current_identity;

fn main() -> std::process::ExitCode {
    run("whoami", real_main)
}

fn real_main() -> AppResult<()> {
    println!("{}", current_identity().user);
    Ok(())
}
