//! `chown` -- change file owner (and optionally group, via
//! `owner:group`). Supports `-R`/`--recursive`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::walk;
use mitos_utils::common::users::{gid_for_name, uid_for_name};
use std::path::Path;

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

fn main() -> std::process::ExitCode {
    run("chown", real_main)
}

fn real_main() -> AppResult<()> {
    let mut recursive = false;
    let mut rest: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-R" | "--recursive" => recursive = true,
            _ => rest.push(arg),
        }
    }
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
            gid_for_name(name).ok_or_else(|| AppError::new(format!("invalid group: '{}'", name)))?,
        ),
        None => None,
    };

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        let targets = if recursive {
            walk(path).map_err(|e| AppError::new(e.to_string()))?
        } else {
            vec![path.to_path_buf()]
        };
        for target in &targets {
            if let Err(err) = chown_one(target, uid, gid) {
                error_path("chown", target.display(), err);
                had_error = true;
            }
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn chown_one(path: &Path, uid: Option<u32>, gid: Option<u32>) -> std::io::Result<()> {
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    // -1 (as u32::MAX) tells chown(2) to leave that ID unchanged.
    let owner = uid.unwrap_or(u32::MAX);
    let group = gid.unwrap_or(u32::MAX);
    let result = unsafe { ffi::chown(c_path.as_ptr(), owner, group) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn chown_one(_path: &Path, _uid: Option<u32>, _gid: Option<u32>) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "chown not available on this target"))
}
