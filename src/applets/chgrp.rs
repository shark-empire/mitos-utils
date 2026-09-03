//! `chgrp` -- change a file's group. Supports `-R`/`--recursive`.
//! Recursive mode on Linux goes through `common::safewalk` for
//! TOCTOU-safe traversal (see that module's docs); other targets use
//! the plain path-based walk.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::error_path;
#[cfg(not(target_os = "linux"))]
use crate::common::paths::walk;
use crate::common::users::gid_for_name;
use std::path::Path;

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

pub const USAGE: &str = "chgrp [-R] GROUP FILE... -- change file group";

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
        return Err(AppError::usage("usage: chgrp [-R] GROUP FILE..."));
    }
    let group_name = rest.remove(0);
    let gid = gid_for_name(&group_name)
        .ok_or_else(|| AppError::new(format!("invalid group: '{}'", group_name)))?;
    let files = rest;

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        if let Err(err) = chgrp_one(path, gid, recursive) {
            error_path("chgrp", file, err);
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
fn chgrp_one(path: &Path, gid: u32, recursive: bool) -> std::io::Result<()> {
    if recursive && path.is_dir() && !path.is_symlink() {
        return crate::common::safewalk::chown_tree(path, u32::MAX, gid);
    }
    chgrp_path(path, gid)
}

#[cfg(not(target_os = "linux"))]
fn chgrp_one(path: &Path, gid: u32, recursive: bool) -> std::io::Result<()> {
    let targets = if recursive { walk(path)? } else { vec![path.to_path_buf()] };
    for target in &targets {
        chgrp_path(target, gid)?;
    }
    Ok(())
}

#[cfg(unix)]
fn chgrp_path(path: &Path, gid: u32) -> std::io::Result<()> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let result = unsafe { ffi::chown(c_path.as_ptr(), u32::MAX, gid) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn chgrp_path(_path: &Path, _gid: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "chgrp not available on this target"))
}
