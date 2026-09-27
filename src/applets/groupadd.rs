//! `groupadd` -- create a new group.
//!
//! Security status: see `common::accounts`'s doc comment -- same
//! weight as `su`/`sudo`. Not audited, not run on a real system.

use crate::common::accounts;
use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "groupadd GROUP -- create a new group";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);
    let name = parts
        .into_iter()
        .next()
        .ok_or_else(|| AppError::usage("missing group name"))?;

    accounts::validate_username(&name).map_err(AppError::new)?;
    if accounts::groupname_exists(&name) {
        return Err(AppError::new(format!("group '{name}' already exists")));
    }

    let _lock = accounts::AccountsLock::acquire()
        .map_err(|e| AppError::new(format!("cannot lock account database: {e}")))?;

    let gid = accounts::next_free_gid()
        .map_err(|e| AppError::new(format!("cannot read /etc/group: {e}")))?;

    let mut contents = std::fs::read_to_string("/etc/group")
        .map_err(|e| AppError::new(format!("cannot read /etc/group: {e}")))?;
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(&format!("{name}:x:{gid}:\n"));

    accounts::atomic_rewrite("/etc/group", &contents, 0o644)
        .map_err(|e| AppError::new(format!("cannot write /etc/group: {e}")))?;

    println!("Added group '{name}' (GID {gid})");
    Ok(())
}
