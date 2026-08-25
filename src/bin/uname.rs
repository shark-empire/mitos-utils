//! `uname` -- print system information. Supports `-a`/`--all` (the
//! default when no flags are given) and `-s`/`-n`/`-r`/`-m` for
//! individual fields.

use mitos_utils::common::errors::{run, AppResult};

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_char;

    const UTSNAME_LEN: usize = 65;

    #[repr(C)]
    pub struct Utsname {
        pub sysname: [c_char; UTSNAME_LEN],
        pub nodename: [c_char; UTSNAME_LEN],
        pub release: [c_char; UTSNAME_LEN],
        pub version: [c_char; UTSNAME_LEN],
        pub machine: [c_char; UTSNAME_LEN],
        pub domainname: [c_char; UTSNAME_LEN],
    }

    extern "C" {
        pub fn uname(buf: *mut Utsname) -> i32;
    }

    pub unsafe fn field(ptr: *const c_char) -> String {
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

struct Info {
    sysname: String,
    nodename: String,
    release: String,
    version: String,
    machine: String,
}

#[cfg(unix)]
fn gather() -> Info {
    unsafe {
        let mut buf: ffi::Utsname = std::mem::zeroed();
        if ffi::uname(&mut buf) == 0 {
            Info {
                sysname: ffi::field(buf.sysname.as_ptr()),
                nodename: ffi::field(buf.nodename.as_ptr()),
                release: ffi::field(buf.release.as_ptr()),
                version: ffi::field(buf.version.as_ptr()),
                machine: ffi::field(buf.machine.as_ptr()),
            }
        } else {
            fallback()
        }
    }
}

#[cfg(not(unix))]
fn gather() -> Info {
    fallback()
}

fn fallback() -> Info {
    Info {
        sysname: std::env::consts::OS.to_string(),
        nodename: "mitos".to_string(),
        release: env!("CARGO_PKG_VERSION").to_string(),
        version: "mitos-utils".to_string(),
        machine: std::env::consts::ARCH.to_string(),
    }
}

fn main() -> std::process::ExitCode {
    run("uname", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let info = gather();

    if args.is_empty() || args.iter().any(|a| a == "-a" || a == "--all") {
        println!(
            "{} {} {} {} {}",
            info.sysname, info.nodename, info.release, info.version, info.machine
        );
        return Ok(());
    }

    let mut parts = Vec::new();
    for a in &args {
        match a.as_str() {
            "-s" | "--kernel-name" => parts.push(info.sysname.clone()),
            "-n" | "--nodename" => parts.push(info.nodename.clone()),
            "-r" | "--kernel-release" => parts.push(info.release.clone()),
            "-m" | "--machine" => parts.push(info.machine.clone()),
            _ => {}
        }
    }
    println!("{}", parts.join(" "));
    Ok(())
}
