//! `hostname` -- print the system hostname.

use mitos_utils::common::errors::{run, AppError, AppResult};

#[cfg(unix)]
mod ffi {
    use std::os::raw::{c_char, c_int};
    extern "C" {
        pub fn gethostname(name: *mut c_char, len: usize) -> c_int;
    }
}

fn main() -> std::process::ExitCode {
    run("hostname", real_main)
}

fn real_main() -> AppResult<()> {
    println!("{}", lookup().unwrap_or_else(|| "mitos".to_string()));
    Ok(())
}

#[cfg(unix)]
fn lookup() -> Option<String> {
    let mut buf = vec![0u8; 256];
    let ok = unsafe { ffi::gethostname(buf.as_mut_ptr() as *mut _, buf.len()) == 0 };
    if !ok {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    Some(String::from_utf8_lossy(&buf[..end]).into_owned())
}

#[cfg(not(unix))]
fn lookup() -> Option<String> {
    None
}
