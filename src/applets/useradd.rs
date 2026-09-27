//! `useradd` -- create a new user account.
//!
//! Security status: see `common::accounts`'s doc comment -- same
//! weight as `su`/`sudo`. Not audited, not run on a real system.
//!
//! Deliberately narrow: creates a matching private group (same name,
//! a freshly allocated GID) and a home directory (mode 0700, owned
//! by the new user) by default; no `/etc/skel` copying, no `-G`
//! (supplementary groups at creation time), no system-account (`-r`)
//! support. The new account's password starts locked (`!` in
//! `/etc/shadow`) -- run `passwd` for that user separately to
//! actually set one, matching real `useradd`'s own default.

use crate::common::accounts;
use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str =
    "useradd [-d HOME] [-s SHELL] USERNAME -- create a new user account (starts locked)";

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);

    let mut home: Option<String> = None;
    let mut shell = "/bin/sh".to_string();
    let mut username: Option<String> = None;

    let mut iter = parts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-d" => {
                home = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-d requires an argument"))?,
                )
            }
            "-s" => {
                shell = iter
                    .next()
                    .ok_or_else(|| AppError::usage("-s requires an argument"))?
            }
            other if username.is_none() => username = Some(other.to_string()),
            other => return Err(AppError::usage(format!("unexpected argument '{other}'"))),
        }
    }
    let username = username.ok_or_else(|| AppError::usage("missing username"))?;
    accounts::validate_username(&username).map_err(AppError::new)?;
    if accounts::username_exists(&username) {
        return Err(AppError::new(format!("user '{username}' already exists")));
    }
    if accounts::groupname_exists(&username) {
        return Err(AppError::new(format!(
            "a group named '{username}' already exists (won't create a same-named private group)"
        )));
    }
    let home = home.unwrap_or_else(|| format!("/home/{username}"));

    let _lock = accounts::AccountsLock::acquire()
        .map_err(|e| AppError::new(format!("cannot lock account database: {e}")))?;

    let gid = accounts::next_free_gid()
        .map_err(|e| AppError::new(format!("cannot read /etc/group: {e}")))?;
    let uid = accounts::next_free_uid()
        .map_err(|e| AppError::new(format!("cannot read /etc/passwd: {e}")))?;

    let mut group_contents = std::fs::read_to_string("/etc/group")
        .map_err(|e| AppError::new(format!("cannot read /etc/group: {e}")))?;
    if !group_contents.is_empty() && !group_contents.ends_with('\n') {
        group_contents.push('\n');
    }
    group_contents.push_str(&format!("{username}:x:{gid}:\n"));

    let mut passwd_contents = std::fs::read_to_string("/etc/passwd")
        .map_err(|e| AppError::new(format!("cannot read /etc/passwd: {e}")))?;
    if !passwd_contents.is_empty() && !passwd_contents.ends_with('\n') {
        passwd_contents.push('\n');
    }
    passwd_contents.push_str(&format!("{username}:x:{uid}:{gid}::{home}:{shell}\n"));

    let mut shadow_contents = std::fs::read_to_string("/etc/shadow")
        .map_err(|e| AppError::new(format!("cannot read /etc/shadow: {e}")))?;
    if !shadow_contents.is_empty() && !shadow_contents.ends_with('\n') {
        shadow_contents.push('\n');
    }
    let today_days = days_since_epoch();
    // "!" locks the account -- nothing will ever match this hash
    // until a real `passwd` run for this user replaces it.
    shadow_contents.push_str(&format!("{username}:!:{today_days}:0:99999:7:::\n"));

    // Group and passwd first, shadow last: if something fails
    // partway through, an account that exists in passwd but has no
    // shadow entry yet is a more familiar, more recoverable state
    // (it's what a fresh NIS/LDAP-only account already looks like)
    // than the reverse would be.
    accounts::atomic_rewrite("/etc/group", &group_contents, 0o644)
        .map_err(|e| AppError::new(format!("cannot write /etc/group: {e}")))?;
    accounts::atomic_rewrite("/etc/passwd", &passwd_contents, 0o644)
        .map_err(|e| AppError::new(format!("cannot write /etc/passwd: {e}")))?;
    accounts::atomic_rewrite("/etc/shadow", &shadow_contents, 0o600)
        .map_err(|e| AppError::new(format!("cannot write /etc/shadow: {e}")))?;

    create_home(&home, uid, gid)?;

    println!("Added user '{username}' (UID {uid}, GID {gid}), home {home}, shell {shell}");
    println!("Account is locked -- run `passwd {username}` to set a password.");
    Ok(())
}

fn days_since_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0)
}

#[cfg(unix)]
fn create_home(home: &str, uid: u32, gid: u32) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(home)
        .map_err(|e| AppError::new(format!("cannot create home directory '{home}': {e}")))?;
    std::fs::set_permissions(home, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| AppError::new(format!("cannot set home directory permissions: {e}")))?;
    let c_home = std::ffi::CString::new(home)
        .map_err(|e| AppError::new(format!("invalid home path: {e}")))?;
    if unsafe { ffi::chown(c_home.as_ptr(), uid, gid) } != 0 {
        return Err(AppError::new(format!(
            "cannot set home directory ownership: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn create_home(_home: &str, _uid: u32, _gid: u32) -> AppResult<()> {
    Err(AppError::new(
        "creating a home directory not available on this target",
    ))
}
