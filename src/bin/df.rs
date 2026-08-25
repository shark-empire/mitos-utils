//! `df` -- report filesystem disk space usage via `statvfs(2)`.
//!
//! The `statvfs` struct layout below matches glibc on Linux/x86_64
//! (the widely-used `libc` crate defines the same fields in the same
//! order) -- it is Linux-specific by construction, matching this
//! crate's zero-external-dependency approach (docs/architecture.md).
//! macOS/BSD `statvfs` uses a different field layout and is
//! deliberately not supported. As with the rest of this crate: no
//! Rust toolchain was available to compile-test this against a real
//! glibc header, so verify the offsets against `<sys/statvfs.h>`
//! before shipping.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::{error_path, human_size};

#[cfg(target_os = "linux")]
mod ffi {
    use std::os::raw::c_char;

    #[repr(C)]
    pub struct Statvfs {
        pub f_bsize: u64,
        pub f_frsize: u64,
        pub f_blocks: u64,
        pub f_bfree: u64,
        pub f_bavail: u64,
        pub f_files: u64,
        pub f_ffree: u64,
        pub f_favail: u64,
        pub f_fsid: u64,
        pub f_flag: u64,
        pub f_namemax: u64,
        pub f_spare: [i32; 6],
    }

    extern "C" {
        pub fn statvfs(path: *const c_char, buf: *mut Statvfs) -> i32;
    }
}

fn main() -> std::process::ExitCode {
    run("df", real_main)
}

#[cfg(target_os = "linux")]
fn real_main() -> AppResult<()> {
    let targets: Vec<String> = std::env::args().skip(1).collect();
    let targets = if targets.is_empty() { vec![".".to_string()] } else { targets };

    println!("{:<20} {:>10} {:>10} {:>10}  {}", "Filesystem", "Size", "Used", "Avail", "Mounted on");

    let mut had_error = false;
    for target in &targets {
        match statvfs_of(target) {
            Ok((total, avail)) => {
                let used = total.saturating_sub(avail);
                println!(
                    "{:<20} {:>10} {:>10} {:>10}  {}",
                    "-", human_size(total), human_size(used), human_size(avail), target
                );
            }
            Err(err) => {
                error_path("df", target, err);
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

#[cfg(target_os = "linux")]
fn statvfs_of(path: &str) -> std::io::Result<(u64, u64)> {
    let c_path = std::ffi::CString::new(path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;
    let mut buf: ffi::Statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { ffi::statvfs(c_path.as_ptr(), &mut buf) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let block_size = if buf.f_frsize > 0 { buf.f_frsize } else { buf.f_bsize };
    Ok((buf.f_blocks * block_size, buf.f_bavail * block_size))
}

#[cfg(not(target_os = "linux"))]
fn real_main() -> AppResult<()> {
    Err(AppError::new(
        "filesystem space info not available on this target (mitosOS has no statvfs yet)",
    ))
}
