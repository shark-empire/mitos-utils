//! `dmesg` -- print the kernel ring buffer. Reads it on Linux via
//! `klogctl(2)` (`syslog(2)`'s `SYSLOG_ACTION_READ_ALL`). mitosOS
//! doesn't expose a kernel log buffer to userspace yet (see
//! docs/architecture.md) -- this is written against the Linux
//! interface as the reference shape for what mitosOS's own log
//! syscall should look like.

use mitos_utils::common::errors::{run, AppError, AppResult};

#[cfg(target_os = "linux")]
mod ffi {
    use std::os::raw::{c_char, c_int};
    extern "C" {
        pub fn klogctl(kind: c_int, buf: *mut c_char, len: c_int) -> c_int;
    }
}

fn main() -> std::process::ExitCode {
    run("dmesg", real_main)
}

#[cfg(target_os = "linux")]
fn real_main() -> AppResult<()> {
    const SYSLOG_ACTION_READ_ALL: i32 = 3;
    let mut buf = vec![0u8; 256 * 1024];
    let n = unsafe {
        ffi::klogctl(
            SYSLOG_ACTION_READ_ALL,
            buf.as_mut_ptr() as *mut _,
            buf.len() as i32,
        )
    };
    if n < 0 {
        return Err(AppError::new(format!(
            "cannot read kernel log: {}",
            std::io::Error::last_os_error()
        )));
    }
    print!("{}", String::from_utf8_lossy(&buf[..n as usize]));
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn real_main() -> AppResult<()> {
    Err(AppError::new(
        "kernel log not available on this target (mitosOS has no log syscall yet)",
    ))
}
