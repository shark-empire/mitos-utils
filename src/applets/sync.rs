//! `sync` -- flush filesystem buffers to disk via the `sync(2)`
//! syscall.

use crate::common::errors::AppResult;

#[cfg(unix)]
mod ffi {
    extern "C" {
        pub fn sync();
    }
}

pub const USAGE: &str = "sync -- flush filesystem buffers to disk";

#[cfg(unix)]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    unsafe { ffi::sync() };
    Ok(())
}

#[cfg(not(unix))]
pub fn run(_args: Vec<String>) -> AppResult<()> {
    Err(AppError::new("sync not available on this target"))
}
