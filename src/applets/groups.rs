//! `groups` -- print the names of the current user's primary and
//! supplementary groups.

use crate::common::errors::AppResult;
use crate::common::users::{current_identity, name_for_gid, supplementary_groups};

pub const USAGE: &str = "groups -- print the current user's groups";

pub fn run(_args: Vec<String>) -> AppResult<()> {
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
