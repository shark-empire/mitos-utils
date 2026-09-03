//! `id` -- print real user/group IDs and names, plus supplementary
//! groups, for the current user.

use crate::common::errors::AppResult;
use crate::common::users::{current_identity, name_for_gid, supplementary_groups};

pub const USAGE: &str = "id -- print uid/gid/groups for the current user";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    let identity = current_identity();
    let groups = supplementary_groups();
    let group_list: Vec<String> = groups
        .iter()
        .map(|&gid| {
            let name = name_for_gid(gid).unwrap_or_else(|| gid.to_string());
            format!("{}({})", gid, name)
        })
        .collect();

    println!(
        "uid={}({}) gid={}({}) groups={}",
        identity.uid,
        identity.user,
        identity.gid,
        identity.group,
        if group_list.is_empty() {
            format!("{}({})", identity.gid, identity.group)
        } else {
            group_list.join(",")
        }
    );
    Ok(())
}
