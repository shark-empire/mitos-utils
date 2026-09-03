//! `mount` -- with no arguments, lists currently mounted filesystems
//! (reads Linux's `/proc/mounts`). With `SOURCE TARGET -t FSTYPE`,
//! attempts a real `mount(2)` call. mitosOS doesn't have a mount
//! table exposed to userspace yet (see docs/architecture.md).

use crate::common::errors::{AppError, AppResult};

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;
    extern "C" {
        pub fn mount(
            source: *const c_char,
            target: *const c_char,
            filesystemtype: *const c_char,
            mountflags: u64,
            data: *const c_char,
        ) -> i32;
    }
}

pub const USAGE: &str = "mount | mount SOURCE TARGET -t FSTYPE -- list or create mounts";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return list_mounts();
    }

    let mut fstype: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "-t" {
            fstype = iter.next();
        } else {
            rest.push(arg);
        }
    }
    if rest.len() != 2 {
        return Err(AppError::usage("usage: mount SOURCE TARGET -t FSTYPE"));
    }

    do_mount(&rest[0], &rest[1], fstype.as_deref().unwrap_or("auto"))
}

#[cfg(target_os = "linux")]
fn list_mounts() -> AppResult<()> {
    let content = std::fs::read_to_string("/proc/mounts")
        .map_err(|e| AppError::new(format!("cannot read /proc/mounts: {}", e)))?;
    print!("{}", content);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn list_mounts() -> AppResult<()> {
    Err(AppError::new(
        "mount table not available on this target (mitosOS has no mount table yet)",
    ))
}

#[cfg(unix)]
fn do_mount(source: &str, target: &str, fstype: &str) -> AppResult<()> {
    let c_source = std::ffi::CString::new(source).map_err(|_| AppError::usage("invalid source"))?;
    let c_target = std::ffi::CString::new(target).map_err(|_| AppError::usage("invalid target"))?;
    let c_fstype = std::ffi::CString::new(fstype).map_err(|_| AppError::usage("invalid fstype"))?;
    let result = unsafe {
        ffi::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(AppError::new(format!(
            "mounting {} on {} failed: {}",
            source,
            target,
            std::io::Error::last_os_error()
        )))
    }
}

#[cfg(not(unix))]
fn do_mount(_source: &str, _target: &str, _fstype: &str) -> AppResult<()> {
    Err(AppError::new("mount not available on this target"))
}
