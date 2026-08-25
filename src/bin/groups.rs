//! `groups` -- print the names of the current user's primary and
//! supplementary groups.

use mitos_utils::common::errors::{run, AppResult};
use mitos_utils::common::users::{current_identity, name_for_gid, supplementary_groups};

fn main() -> std::process::ExitCode {
    run("groups", real_main)
}

fn real_main() -> AppResult<()> {
    let identity = current_identity();
    let mut names = vec![identity.group.clone()];
    for gid in supplementary_groups() {
        if gid != identity.gid {
            names.push(name_for_gid(gid).unwrap_or_else(|| gid.to_string()));
        }
    }
    println!("{}", names.join(" "));
    Ok(())
}
