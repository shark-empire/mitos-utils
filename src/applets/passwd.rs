//! `passwd` -- change a user's password.
//!
//! Security status: see `common::auth` and `common::accounts`'s doc
//! comments -- generating and writing a new password hash carries
//! the same weight as `su`/`sudo`'s verification side. Not audited,
//! not run on a real system.
//!
//! Deliberately narrow: no password-strength/complexity checking, no
//! password-history/reuse checking, no expiry-policy enforcement.
//! Real gaps, not silent ones.

use crate::common::accounts;
use crate::common::auth;
use crate::common::errors::{AppError, AppResult};
use crate::common::users;

pub const USAGE: &str = "passwd [USERNAME] -- change a password (default: your own)";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);

    let caller = users::current_identity();
    let target = parts
        .into_iter()
        .next()
        .unwrap_or_else(|| caller.user.clone());

    if target != caller.user && caller.euid != 0 {
        return Err(AppError::new(format!(
            "only root can change another user's password (tried '{target}')"
        )));
    }
    if !accounts::username_exists(&target) {
        return Err(AppError::new(format!("user '{target}' does not exist")));
    }

    // Changing your own password still means proving you are who you
    // say you are first, same as real passwd -- root changing
    // someone else's doesn't need the target's current password
    // (root can already do anything a password check would gate).
    if caller.euid != 0 {
        auth::authenticate_interactively(&target)?;
    }

    let new_password = auth::prompt_password("New password: ")
        .map_err(|e| AppError::new(format!("cannot read password: {e}")))?;
    let confirm = auth::prompt_password("Retype new password: ")
        .map_err(|e| AppError::new(format!("cannot read password: {e}")))?;
    if new_password.is_empty() {
        return Err(AppError::new("password cannot be empty"));
    }
    if new_password != confirm {
        return Err(AppError::new("passwords do not match"));
    }

    let new_hash = auth::generate_hash(&new_password)
        .map_err(|e| AppError::new(format!("cannot generate password hash: {e}")))?;

    let _lock = accounts::AccountsLock::acquire()
        .map_err(|e| AppError::new(format!("cannot lock account database: {e}")))?;

    let contents = std::fs::read_to_string("/etc/shadow")
        .map_err(|e| AppError::new(format!("cannot read /etc/shadow: {e}")))?;

    let today_days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0);

    let mut found = false;
    let updated_lines: Vec<String> = contents
        .lines()
        .map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 9 && fields[0] == target {
                found = true;
                format!(
                    "{}:{}:{}:{}:{}:{}:{}:{}:{}",
                    fields[0],
                    new_hash,
                    today_days,
                    fields[3],
                    fields[4],
                    fields[5],
                    fields[6],
                    fields[7],
                    fields[8]
                )
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        return Err(AppError::new(format!(
            "user '{target}' has no /etc/shadow entry to update"
        )));
    }

    let mut new_contents = updated_lines.join("\n");
    new_contents.push('\n');

    accounts::atomic_rewrite("/etc/shadow", &new_contents, 0o600)
        .map_err(|e| AppError::new(format!("cannot write /etc/shadow: {e}")))?;

    println!("Password updated for '{target}'.");
    Ok(())
}
