//! `chgrp` -- change a file's group. Supports `-R`/`--recursive`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::walk;
use mitos_utils::common::users::gid_for_name;
use std::path::Path;

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> i32;
    }
}

fn main() -> std::process::ExitCode {
    run("chgrp", real_main)
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
        return Err(AppError::usage("usage: chgrp [-R] GROUP FILE..."));
    }
    let group_name = rest.remove(0);
    let gid = gid_for_name(&group_name)
        .ok_or_else(|| AppError::new(format!("invalid group: '{}'", group_name)))?;
    let files = rest;

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        let targets = if recursive {
            walk(path).map_err(|e| AppError::new(e.to_string()))?
        } else {
            vec![path.to_path_buf()]
        };
        for target in &targets {
            if let Err(err) = chgrp_one(target, gid) {
                error_path("chgrp", target.display(), err);
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
fn chgrp_one(path: &Path, gid: u32) -> std::io::Result<()> {
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
fn chgrp_one(_path: &Path, _gid: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "chgrp not available on this target"))
}
