//! Current-user / group identity lookups used by `whoami`, `id`,
//! `groups`, and for rendering owner/group names in `ls -l` /
//! `stat`.
//!
//! mitosOS doesn't have a hosted passwd/group database or syscalls
//! of its own yet, so on unix this talks to the *host* system's
//! libc directly via a handful of hand-written `extern "C"`
//! declarations, rather than adding the `libc` crate as a
//! dependency -- keeping this workspace at zero external
//! dependencies, matching the rest of mitos-utils (see
//! docs/architecture.md). Once mitosOS grows its own identity
//! syscalls, this module is the only place that needs to change.

pub struct Identity {
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
    pub user: String,
    pub group: String,
}

#[cfg(unix)]
mod ffi {
    use std::os::raw::{c_char, c_int};

    #[repr(C)]
    pub struct Passwd {
        pub pw_name: *const c_char,
        pub pw_passwd: *const c_char,
        pub pw_uid: u32,
        pub pw_gid: u32,
        pub pw_gecos: *const c_char,
        pub pw_dir: *const c_char,
        pub pw_shell: *const c_char,
    }

    #[repr(C)]
    pub struct Group {
        pub gr_name: *const c_char,
        pub gr_passwd: *const c_char,
        pub gr_gid: u32,
        pub gr_mem: *const *const c_char,
    }

    extern "C" {
        pub fn getuid() -> u32;
        pub fn geteuid() -> u32;
        pub fn getgid() -> u32;
        pub fn getegid() -> u32;
        pub fn getpwuid(uid: u32) -> *const Passwd;
        pub fn getgrgid(gid: u32) -> *const Group;
        pub fn getpwnam(name: *const c_char) -> *const Passwd;
        pub fn getgrnam(name: *const c_char) -> *const Group;
        pub fn getgroups(size: c_int, list: *mut u32) -> c_int;
    }
}

#[cfg(unix)]
unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

#[cfg(unix)]
pub fn current_identity() -> Identity {
    unsafe {
        let uid = ffi::getuid();
        let gid = ffi::getgid();
        let euid = ffi::geteuid();
        let egid = ffi::getegid();
        Identity {
            uid,
            gid,
            euid,
            egid,
            user: name_for_uid(uid).unwrap_or_else(|| uid.to_string()),
            group: name_for_gid(gid).unwrap_or_else(|| gid.to_string()),
        }
    }
}

#[cfg(unix)]
pub fn name_for_uid(uid: u32) -> Option<String> {
    unsafe {
        let pw = ffi::getpwuid(uid);
        if pw.is_null() {
            None
        } else {
            cstr_to_string((*pw).pw_name)
        }
    }
}

#[cfg(unix)]
pub fn name_for_gid(gid: u32) -> Option<String> {
    unsafe {
        let gr = ffi::getgrgid(gid);
        if gr.is_null() {
            None
        } else {
            cstr_to_string((*gr).gr_name)
        }
    }
}

/// Look up a uid by username (`chown`).
#[cfg(unix)]
pub fn uid_for_name(name: &str) -> Option<u32> {
    let cname = std::ffi::CString::new(name).ok()?;
    unsafe {
        let pw = ffi::getpwnam(cname.as_ptr());
        if pw.is_null() {
            None
        } else {
            Some((*pw).pw_uid)
        }
    }
}

/// Look up a gid by group name (`chown`, `chgrp`).
#[cfg(unix)]
pub fn gid_for_name(name: &str) -> Option<u32> {
    let cname = std::ffi::CString::new(name).ok()?;
    unsafe {
        let gr = ffi::getgrnam(cname.as_ptr());
        if gr.is_null() {
            None
        } else {
            Some((*gr).gr_gid)
        }
    }
}

#[cfg(not(unix))]
pub fn uid_for_name(_name: &str) -> Option<u32> {
    None
}
#[cfg(not(unix))]
pub fn gid_for_name(_name: &str) -> Option<u32> {
    None
}

/// Supplementary group IDs for the calling process (`groups`, `id -G`).
#[cfg(unix)]
pub fn supplementary_groups() -> Vec<u32> {
    unsafe {
        let n = ffi::getgroups(0, std::ptr::null_mut());
        if n <= 0 {
            return Vec::new();
        }
        let mut buf = vec![0u32; n as usize];
        let n2 = ffi::getgroups(n, buf.as_mut_ptr());
        if n2 < 0 {
            return Vec::new();
        }
        buf.truncate(n2 as usize);
        buf
    }
}

#[cfg(not(unix))]
pub fn current_identity() -> Identity {
    Identity { uid: 0, gid: 0, euid: 0, egid: 0, user: "unknown".into(), group: "unknown".into() }
}
#[cfg(not(unix))]
pub fn name_for_uid(_uid: u32) -> Option<String> {
    None
}
#[cfg(not(unix))]
pub fn name_for_gid(_gid: u32) -> Option<String> {
    None
}
#[cfg(not(unix))]
pub fn supplementary_groups() -> Vec<u32> {
    Vec::new()
}
