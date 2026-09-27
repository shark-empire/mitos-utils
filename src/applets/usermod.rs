//! `usermod` -- modify an existing user account's `/etc/passwd`,
//! `/etc/shadow`, and `/etc/group` fields.
//!
//! Security status: see `common::accounts`'s doc comment -- same
//! weight as `su`/`sudo`/`useradd`/`groupadd`/`passwd`. This mutates
//! the same three files those do, has not been audited, and hasn't
//! been run on a real system.
//!
//! Deliberately narrower than real `usermod`: no `-l` (rename -- a
//! username change would have to ripple through `/etc/shadow`'s key,
//! every group's member list, and conventionally the home directory
//! path too; rather than half-do that, it's left out entirely, same
//! spirit as `useradd` leaving out `-G`/`-r`), no `-o` (permit a
//! duplicate uid), and no automatic re-`chown` of files scattered
//! elsewhere on the filesystem when `-u`/`-g` change an id (real
//! `usermod` doesn't do that either, outside of the home directory
//! itself when combined with `-d -m` -- see below). `-G` without
//! `-a` really does *replace* the full supplementary-group list,
//! matching real `usermod` -- never a silent surprise, since the
//! summary line always names exactly what was added and removed.

use crate::common::accounts;
use crate::common::errors::{AppError, AppResult};
use crate::common::users;
use std::path::Path;

pub const USAGE: &str = "usermod [-c COMMENT] [-d HOME [-m]] [-g GROUP] [-aG GROUPS|-G GROUPS] [-s SHELL] [-u UID] [-L|-U] USERNAME -- modify an existing user account";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut parts = opts;
    parts.extend(forced);

    let mut comment: Option<String> = None;
    let mut home: Option<String> = None;
    let mut move_home = false;
    let mut primary_group: Option<String> = None;
    let mut groups_arg: Option<String> = None;
    let mut append_flag = false;
    let mut shell: Option<String> = None;
    let mut uid_arg: Option<String> = None;
    let mut lock = false;
    let mut unlock = false;
    let mut username: Option<String> = None;

    let mut iter = parts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                comment = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-c requires an argument"))?,
                )
            }
            "-d" => {
                home = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-d requires an argument"))?,
                )
            }
            "-m" => move_home = true,
            "-g" => {
                primary_group = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-g requires an argument"))?,
                )
            }
            "-a" => append_flag = true,
            "-G" => {
                groups_arg = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-G requires an argument"))?,
                )
            }
            // Real usermod's own idiom is "-aG group1,group2" (`-a`
            // clustered with `-G`'s value) -- recognized directly
            // rather than only accepting "-a -G group" split apart,
            // since the clustered form is by far the more common one
            // people actually type.
            "-aG" => {
                append_flag = true;
                groups_arg = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-aG requires an argument"))?,
                )
            }
            "-s" => {
                shell = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-s requires an argument"))?,
                )
            }
            "-u" => {
                uid_arg = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-u requires an argument"))?,
                )
            }
            "-L" => lock = true,
            "-U" => unlock = true,
            other if username.is_none() => username = Some(other.to_string()),
            other => return Err(AppError::usage(format!("unexpected argument '{other}'"))),
        }
    }

    let username = username.ok_or_else(|| AppError::usage("missing username"))?;
    if move_home && home.is_none() {
        return Err(AppError::usage("-m requires -d"));
    }
    if append_flag && groups_arg.is_none() {
        return Err(AppError::usage("-a is only used together with -G"));
    }
    if lock && unlock {
        return Err(AppError::usage("-L and -U are mutually exclusive"));
    }
    if comment.is_none()
        && home.is_none()
        && primary_group.is_none()
        && groups_arg.is_none()
        && shell.is_none()
        && uid_arg.is_none()
        && !lock
        && !unlock
    {
        return Err(AppError::usage("no changes requested"));
    }
    for field in [&comment, &home, &shell] {
        if let Some(v) = field {
            accounts::validate_field(v).map_err(AppError::new)?;
        }
    }

    let caller = users::current_identity();
    if caller.euid != 0 {
        return Err(AppError::new(
            "permission denied -- usermod must run as root (setuid-root); it writes \
             /etc/passwd, /etc/shadow, and /etc/group directly",
        ));
    }
    if !accounts::username_exists(&username) {
        return Err(AppError::new(format!("user '{username}' does not exist")));
    }

    let new_gid = match &primary_group {
        Some(g) => Some(
            accounts::gid_for_groupname(g)
                .ok_or_else(|| AppError::new(format!("group '{g}' does not exist")))?,
        ),
        None => None,
    };
    let new_uid: Option<u32> = match &uid_arg {
        Some(u) => Some(
            u.parse()
                .map_err(|_| AppError::usage(format!("invalid uid '{u}'")))?,
        ),
        None => None,
    };

    let resolve_group_list = |raw: &str| -> AppResult<Vec<String>> {
        let names: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for name in &names {
            if !accounts::groupname_exists(name) {
                return Err(AppError::new(format!("group '{name}' does not exist")));
            }
        }
        Ok(names)
    };
    let (append_groups, replace_groups) = match &groups_arg {
        Some(raw) if append_flag => (Some(resolve_group_list(raw)?), None),
        Some(raw) => (None, Some(resolve_group_list(raw)?)),
        None => (None, None),
    };

    let _lock_guard = accounts::AccountsLock::acquire()
        .map_err(|e| AppError::new(format!("cannot lock account database: {e}")))?;

    // Re-read (rather than trust the pre-lock check above) now that
    // the lock is actually held -- the account could in principle
    // have been removed by something else in between.
    let passwd_contents = std::fs::read_to_string("/etc/passwd")
        .map_err(|e| AppError::new(format!("cannot read /etc/passwd: {e}")))?;
    let old_line = passwd_contents
        .lines()
        .find(|l| l.split(':').next() == Some(username.as_str()))
        .ok_or_else(|| AppError::new(format!("user '{username}' does not exist")))?;
    let fields: Vec<&str> = old_line.split(':').collect();
    if fields.len() < 7 {
        return Err(AppError::new(format!(
            "malformed /etc/passwd entry for '{username}'"
        )));
    }
    let old_uid: u32 = fields[2]
        .parse()
        .map_err(|_| AppError::new(format!("malformed uid in /etc/passwd for '{username}'")))?;
    let old_gid: u32 = fields[3]
        .parse()
        .map_err(|_| AppError::new(format!("malformed gid in /etc/passwd for '{username}'")))?;
    let old_home = fields[5].to_string();
    let old_shell = fields[6].to_string();
    let old_comment = fields[4].to_string();
    let passwd_x = fields[1].to_string();

    if let Some(u) = new_uid {
        if u != old_uid {
            let taken = passwd_contents.lines().any(|l| {
                let f: Vec<&str> = l.split(':').collect();
                f.len() > 2 && f[0] != username && f[2] == u.to_string()
            });
            if taken {
                return Err(AppError::new(format!("uid {u} is already in use")));
            }
        }
    }

    let final_uid = new_uid.unwrap_or(old_uid);
    let final_gid = new_gid.unwrap_or(old_gid);
    let final_home = home.clone().unwrap_or_else(|| old_home.clone());
    let final_shell = shell.clone().unwrap_or_else(|| old_shell.clone());
    let final_comment = comment.clone().unwrap_or(old_comment);

    // The riskier filesystem move happens *before* any account-file
    // is rewritten: if it fails, `/etc/passwd` never ends up pointing
    // at a home directory that doesn't actually exist there.
    if move_home && old_home != final_home && Path::new(&old_home).is_dir() {
        std::fs::rename(&old_home, &final_home).map_err(|e| {
            AppError::new(format!(
                "cannot move home directory '{old_home}' to '{final_home}': {e} \
                 (cross-filesystem moves aren't supported by usermod -m -- move it \
                 manually with `mv`, then rerun usermod -d without -m)"
            ))
        })?;
        if final_uid != old_uid || final_gid != old_gid {
            chown_moved_home(Path::new(&final_home), final_uid, final_gid).map_err(|e| {
                AppError::new(format!(
                    "moved '{old_home}' to '{final_home}' but could not update its ownership: {e}"
                ))
            })?;
        }
    }

    let new_passwd_line = format!(
        "{username}:{passwd_x}:{final_uid}:{final_gid}:{final_comment}:{final_home}:{final_shell}"
    );
    let mut passwd_lines_out: Vec<String> = Vec::new();
    for l in passwd_contents.lines() {
        if l.split(':').next() == Some(username.as_str()) {
            passwd_lines_out.push(new_passwd_line.clone());
        } else {
            passwd_lines_out.push(l.to_string());
        }
    }
    let new_passwd_contents = passwd_lines_out.join("\n") + "\n";
    accounts::atomic_rewrite("/etc/passwd", &new_passwd_contents, 0o644)
        .map_err(|e| AppError::new(format!("cannot write /etc/passwd: {e}")))?;

    let mut group_summary = String::new();
    if append_groups.is_some() || replace_groups.is_some() {
        let group_contents = std::fs::read_to_string("/etc/group")
            .map_err(|e| AppError::new(format!("cannot read /etc/group: {e}")))?;
        let mut added: Vec<String> = Vec::new();
        let mut removed: Vec<String> = Vec::new();
        let mut group_lines_out: Vec<String> = Vec::new();

        for line in group_contents.lines() {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 4 {
                group_lines_out.push(line.to_string());
                continue;
            }
            let gname = f[0];
            let mut members: Vec<String> = if f[3].is_empty() {
                Vec::new()
            } else {
                f[3].split(',').map(|s| s.to_string()).collect()
            };
            let is_member = members.iter().any(|m| m == &username);

            let should_be_member = if let Some(list) = &append_groups {
                is_member || list.iter().any(|g| g == gname)
            } else if let Some(list) = &replace_groups {
                list.iter().any(|g| g == gname)
            } else {
                is_member
            };

            if should_be_member && !is_member {
                members.push(username.clone());
                added.push(gname.to_string());
            } else if !should_be_member && is_member {
                members.retain(|m| m != &username);
                removed.push(gname.to_string());
            }

            group_lines_out.push(format!("{}:{}:{}:{}", f[0], f[1], f[2], members.join(",")));
        }

        let new_group_contents = group_lines_out.join("\n") + "\n";
        accounts::atomic_rewrite("/etc/group", &new_group_contents, 0o644)
            .map_err(|e| AppError::new(format!("cannot write /etc/group: {e}")))?;

        if !added.is_empty() {
            group_summary.push_str(&format!(" Added to: {}.", added.join(", ")));
        }
        if !removed.is_empty() {
            group_summary.push_str(&format!(" Removed from: {}.", removed.join(", ")));
        }
    }

    if lock || unlock {
        let shadow_contents = std::fs::read_to_string("/etc/shadow")
            .map_err(|e| AppError::new(format!("cannot read /etc/shadow: {e}")))?;

        let mut found = false;
        let mut already_in_state = false;
        let mut refused_unlock = false;

        let updated_lines: Vec<String> = shadow_contents
            .lines()
            .map(|line| {
                let f: Vec<&str> = line.split(':').collect();
                if f.len() < 2 || f[0] != username {
                    return line.to_string();
                }
                found = true;
                let hash = f[1];
                let new_hash: String = if lock {
                    if let Some(rest) = hash.strip_prefix('!') {
                        already_in_state = true;
                        format!("!{rest}")
                    } else {
                        format!("!{hash}")
                    }
                } else if let Some(rest) = hash.strip_prefix('!') {
                    if rest.is_empty() {
                        refused_unlock = true;
                        hash.to_string()
                    } else {
                        rest.to_string()
                    }
                } else {
                    already_in_state = true;
                    hash.to_string()
                };
                let mut out_fields: Vec<String> = f.iter().map(|s| s.to_string()).collect();
                out_fields[1] = new_hash;
                out_fields.join(":")
            })
            .collect();

        if !found {
            return Err(AppError::new(format!(
                "user '{username}' has no /etc/shadow entry"
            )));
        }
        if refused_unlock {
            return Err(AppError::new(format!(
                "cannot unlock '{username}': no password is set (the account was never \
                 given one) -- run `passwd {username}` first"
            )));
        }

        let new_shadow_contents = updated_lines.join("\n") + "\n";
        accounts::atomic_rewrite("/etc/shadow", &new_shadow_contents, 0o600)
            .map_err(|e| AppError::new(format!("cannot write /etc/shadow: {e}")))?;

        println!(
            "Account '{username}' {}{}.",
            if already_in_state { "was already " } else { "" },
            if lock { "locked" } else { "unlocked" }
        );
    }

    println!("Modified user '{username}'.{group_summary}");
    Ok(())
}

/// Recursive ownership fixup for a home directory just moved by
/// `-d -m` when `-u`/`-g` also changed in the same invocation.
/// Delegates to `common::safewalk`'s already-TOCTOU-hardened
/// `chown_tree` (the same primitive `chown -R` uses) rather than a
/// second, less-scrutinized recursive walk written just for this.
#[cfg(target_os = "linux")]
fn chown_moved_home(path: &Path, uid: u32, gid: u32) -> std::io::Result<()> {
    crate::common::safewalk::chown_tree(path, uid, gid)
}

#[cfg(not(target_os = "linux"))]
fn chown_moved_home(_path: &Path, _uid: u32, _gid: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "recursive ownership update after a home-directory move is only implemented on Linux",
    ))
}
