//! `umount` -- unmount a filesystem via `umount(2)`.

use crate::common::errors::{AppError, AppResult};

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn umount(target: *const c_char) -> i32;
    }
}

pub const USAGE: &str = "umount TARGET -- unmount a filesystem";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let target = args
        .into_iter()
        .next()
        .ok_or_else(|| AppError::usage("usage: umount TARGET"))?;
    do_umount(&target)
}

#[cfg(unix)]
fn do_umount(target: &str) -> AppResult<()> {
    let c_target = std::ffi::CString::new(target).map_err(|_| AppError::usage("invalid target"))?;
    let result = unsafe { ffi::umount(c_target.as_ptr()) };
    if result == 0 {
        Ok(())
    } else {
        Err(AppError::new(format!(
            "unmounting {} failed: {}",
            target,
            std::io::Error::last_os_error()
        )))
    }
}

#[cfg(not(unix))]
fn do_umount(_target: &str) -> AppResult<()> {
    Err(AppError::new("umount not available on this target"))
}
