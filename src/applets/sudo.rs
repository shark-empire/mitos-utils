//! `sudo` -- run one command as root, after verifying the caller's
//! own password and that they're listed in `/etc/mitos-sudoers`.
//!
//! # Security status
//! See `common::auth`'s doc comment -- the same caveats apply here in
//! full; this is not a lightly-tested utility like the other ~54.
//! This binary only does anything at all if it's installed
//! setuid-root (owned by root, `chmod u+s`); until then, reading
//! `/etc/shadow` simply fails, which is the safe default.
//!
//! Deliberately much smaller than real `sudo`: one flat allowlist
//! (`/etc/mitos-sudoers`, one username per line, `#` comments), no
//! per-command rules, no `NOPASSWD`, no timestamp-based "don't ask
//! again for N minutes" caching, always targets root (no `-u OTHER`).
//! Every one of those is a real gap, not a silent one -- each is
//! also a place real sudoers-parsing/timestamp-cache implementations
//! have had genuine CVEs, so smaller here is deliberate, not just
//! unfinished.

use crate::common::auth;
use crate::common::errors::{AppError, AppResult};
use crate::common::users;
use std::os::unix::process::CommandExt;

pub const USAGE: &str = "sudo COMMAND [ARGS...] -- run COMMAND as root";

const SUDOERS_PATH: &str = "/etc/mitos-sudoers";

/// `/etc/mitos-sudoers` is only meaningful if a non-root user can't
/// edit their own way onto the allowlist -- refuses everyone (fails
/// closed) rather than trusting a file that isn't root-owned and
/// isn't at least closed to group/other writes. Real `sudo` performs
/// the same kind of check on its own sudoers file for the same
/// reason.
fn sudoers_looks_safe(path: &str) -> bool {
    use std::os::unix::fs::MetadataExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.uid() == 0 && meta.mode() & 0o022 == 0,
        Err(_) => false,
    }
}

fn is_authorized(username: &str) -> Result<bool, String> {
    if !sudoers_looks_safe(SUDOERS_PATH) {
        return Err(format!(
            "{SUDOERS_PATH} is missing, not owned by root, or writable by \
             someone other than root -- refusing to trust it"
        ));
    }
    let contents = std::fs::read_to_string(SUDOERS_PATH)
        .map_err(|e| format!("cannot read {SUDOERS_PATH}: {e}"))?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .any(|line| line == username))
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut command_parts = opts;
    command_parts.extend(forced);

    let Some((program, rest)) = command_parts.split_first() else {
        return Err(AppError::usage("missing command"));
    };

    let caller = users::current_identity();

    if caller.euid != 0 {
        match is_authorized(&caller.user) {
            Ok(true) => {}
            Ok(false) => {
                return Err(AppError::with_code(
                    format!(
                        "Sorry, user {} is not allowed to run sudo on this system.",
                        caller.user
                    ),
                    1,
                ));
            }
            Err(reason) => return Err(AppError::new(reason)),
        }
        auth::authenticate_interactively(&caller.user)?;
    }

    auth::drop_privileges_to(0, 0, "root")
        .map_err(|e| AppError::new(format!("cannot become root: {e}")))?;

    // Small allowlist instead of trying to replicate real sudo's
    // env_reset denylist exactly: HOME/TERM are harmless and often
    // needed, everything else starts clear so nothing like
    // LD_PRELOAD or a hijacked PATH rides along into a root process.
    //
    // SAFETY: set_var/remove_var are only unsound when another
    // thread could be reading/writing the environment concurrently;
    // this process is single-threaded up to this point and nothing
    // above spawned another thread, so that precondition holds. Keys
    // are collected into an owned Vec before removing any of them,
    // rather than mutating while iterating std::env::vars() directly.
    let home = std::env::var("HOME").ok();
    let term = std::env::var("TERM").ok();
    let keys: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
    unsafe {
        for key in keys {
            std::env::remove_var(key);
        }
        if let Some(home) = home {
            std::env::set_var("HOME", home);
        }
        if let Some(term) = term {
            std::env::set_var("TERM", term);
        }
        std::env::set_var("USER", "root");
        std::env::set_var("LOGNAME", "root");
        std::env::set_var("SUDO_USER", &caller.user);
        std::env::set_var(
            "PATH",
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        );
    }

    let err = std::process::Command::new(program).args(rest).exec();
    // exec() only returns on failure.
    Err(AppError::new(format!("cannot run '{program}': {err}")))
}
