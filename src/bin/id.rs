//! `id` -- print real user/group IDs and names, plus supplementary
//! groups, for the current user.

use mitos_utils::common::errors::{run, AppResult};
use mitos_utils::common::users::{current_identity, name_for_gid, supplementary_groups};

fn main() -> std::process::ExitCode {
    run("id", real_main)
}

fn real_main() -> AppResult<()> {
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
