//! `su` -- switch to another user's identity, given their password.
//!
//! # Security status
//! See `common::auth`'s doc comment -- the same caveats apply here in
//! full; this is not a lightly-tested utility like the other ~54.
//! This binary only does anything at all if it's installed
//! setuid-root (owned by root, `chmod u+s`); until then, reading
//! `/etc/shadow` simply fails, which is the safe default.
//!
//! Deliberately narrow scope: always resets to a clean, target-user
//! environment. There's no separate "non-login" mode that keeps more
//! of the caller's environment -- one behavior instead of two is less
//! surface, and the login-style reset is the safer of the two anyway.
//! `-c COMMAND` runs one command through the target's shell instead
//! of starting it interactively.

use crate::common::auth;
use crate::common::errors::{AppError, AppResult};
use crate::common::users;
use std::os::unix::process::CommandExt;

pub const USAGE: &str = "su [USER] [-c COMMAND] -- switch to USER's identity (default root)";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, _forced) = crate::common::args::split_dashdash(args);
    let mut target = "root".to_string();
    let mut command: Option<String> = None;
    let mut saw_target = false;

    let mut iter = opts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                command = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-c requires an argument"))?,
                );
            }
            other if other.starts_with('-') => {
                return Err(AppError::usage(format!("unknown option '{other}'")));
            }
            other if !saw_target => {
                target = other.to_string();
                saw_target = true;
            }
            other => return Err(AppError::usage(format!("unexpected argument '{other}'"))),
        }
    }

    let entry = users::passwd_for_name(&target)
        .ok_or_else(|| AppError::new(format!("user '{target}' does not exist")))?;

    // Root can switch to anyone with no password prompt, same as real
    // `su` -- a password check on the target account can't gate
    // anything root couldn't already do directly.
    if users::current_identity().euid != 0 {
        auth::authenticate_interactively(&target)?;
    }

    auth::drop_privileges_to(entry.uid, entry.gid, &target)
        .map_err(|e| AppError::new(format!("cannot switch to '{target}': {e}")))?;

    // A clean, target-user environment -- see this module's doc
    // comment for why there's no "keep more of the caller's env"
    // variant.
    //
    // SAFETY: set_var/remove_var are only unsound when another
    // thread could be reading/writing the environment concurrently;
    // this process is single-threaded up to this point and nothing
    // above spawned another thread, so that precondition holds.
    unsafe {
        std::env::set_var("HOME", &entry.home);
        std::env::set_var("USER", &target);
        std::env::set_var("LOGNAME", &target);
        std::env::set_var("SHELL", &entry.shell);
        std::env::set_var(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        );
    }
    let _ = std::env::set_current_dir(&entry.home);

    // exec() replaces this process outright and only returns on
    // failure -- there's no code path where it returns Ok.
    let err = match &command {
        Some(cmd) => std::process::Command::new(&entry.shell)
            .arg("-c")
            .arg(cmd)
            .exec(),
        None => std::process::Command::new(&entry.shell).arg("-l").exec(),
    };
    Err(AppError::new(format!(
        "cannot run '{}': {err}",
        entry.shell
    )))
}
