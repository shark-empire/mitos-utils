//! `sync` -- flush filesystem buffers to disk via the `sync(2)`
//! syscall.

use mitos_utils::common::errors::{run, AppError, AppResult};

#[cfg(unix)]
mod ffi {
    extern "C" {
        pub fn sync();
    }
}

fn main() -> std::process::ExitCode {
    run("sync", real_main)
}

#[cfg(unix)]
fn real_main() -> AppResult<()> {
    unsafe { ffi::sync() };
    Ok(())
}

#[cfg(not(unix))]
fn real_main() -> AppResult<()> {
    Err(AppError::new("sync not available on this target"))
}
