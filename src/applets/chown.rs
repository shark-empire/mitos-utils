//! `chown` -- change file owner (and optionally group, via
//! `owner:group`). Supports `-R`/`--recursive`. Recursive mode on
//! Linux goes through `common::safewalk` for TOCTOU-safe traversal
//! (see that module's docs); other targets use the plain path-based
//! walk.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
#[cfg(not(target_os = "linux"))]
use crate::common::paths::walk;
use crate::common::users::{gid_for_name, uid_for_name};
use std::path::Path;

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

pub const USAGE: &str = "chown [-R] OWNER[:GROUP] FILE... -- change file owner/group";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut recursive = false;
    let mut rest: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        match arg.as_str() {
            "-R" | "--recursive" => recursive = true,
            _ => rest.push(arg),
        }
    }
    rest.extend(forced);
    if rest.len() < 2 {
        return Err(AppError::usage("usage: chown [-R] OWNER[:GROUP] FILE..."));
    }
    let spec = rest.remove(0);
    let files = rest;

    let (owner_name, group_name) = match spec.split_once(':') {
        Some((o, g)) => (Some(o), Some(g).filter(|g| !g.is_empty())),
        None => (Some(spec.as_str()), None),
    };
    let uid = match owner_name {
        Some(name) if !name.is_empty() => Some(
            uid_for_name(name).ok_or_else(|| AppError::new(format!("invalid user: '{}'", name)))?,
        ),
        _ => None,
    };
    let gid = match group_name {
        Some(name) => Some(
            gid_for_name(name)
                .ok_or_else(|| AppError::new(format!("invalid group: '{}'", name)))?,
        ),
        None => None,
    };

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        if let Err(err) = chown_one(path, uid, gid, recursive) {
            error_path("chown", file, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn chown_one(
    path: &Path,
    uid: Option<u32>,
    gid: Option<u32>,
    recursive: bool,
) -> std::io::Result<()> {
    let owner = uid.unwrap_or(u32::MAX);
    let group = gid.unwrap_or(u32::MAX);
    if recursive && path.is_dir() && !path.is_symlink() {
        return crate::common::safewalk::chown_tree(path, owner, group);
    }
    chown_path(path, owner, group)
}

#[cfg(not(target_os = "linux"))]
fn chown_one(
    path: &Path,
    uid: Option<u32>,
    gid: Option<u32>,
    recursive: bool,
) -> std::io::Result<()> {
    let owner = uid.unwrap_or(u32::MAX);
    let group = gid.unwrap_or(u32::MAX);
    let targets = if recursive {
        walk(path)?
    } else {
        vec![path.to_path_buf()]
    };
    for target in &targets {
        chown_path(target, owner, group)?;
    }
    Ok(())
}

#[cfg(unix)]
fn chown_path(path: &Path, owner: u32, group: u32) -> std::io::Result<()> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let result = unsafe { ffi::chown(c_path.as_ptr(), owner, group) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn chown_path(_path: &Path, _owner: u32, _group: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "chown not available on this target",
    ))
}
