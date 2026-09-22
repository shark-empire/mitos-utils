//! Password verification and privilege-switching primitives shared by
//! `su` and `sudo`.
//!
//! # Security status -- read this before relying on it
//!
//! This has not been audited, and hasn't been run on a real system --
//! same "no toolchain in this environment" caveat as the rest of this
//! crate (see README.md), but it matters far more here: a bug in `cat`
//! is annoying, a bug in this file can mean any local user becomes
//! root. Before installing this setuid-root anywhere real:
//! - Get it reviewed by someone who's spent time reading real CVEs
//!   against `su`/`sudo`/`login` -- privilege-management code has a
//!   long, well-documented history of subtle mistakes.
//! - Test it exhaustively in a disposable VM first: wrong passwords,
//!   locked/`!`-prefixed accounts, a missing or malformed
//!   `/etc/shadow` entry, no controlling terminal, `Ctrl-C` mid-prompt.
//! - Treat every early return below as security-critical: on any
//!   error or missing data, the default must be "deny", never "allow".
//!
//! # Design choices, and why
//!
//! Password hashes are checked via the system's own `crypt(3)`, not a
//! hand-rolled hash function -- the one place this crate's usual
//! zero-dependency-and-hand-roll-the-syscall stance doesn't extend to
//! reimplementing the *algorithm* itself: password hashing is exactly
//! the kind of thing to never write from scratch, and `crypt()`
//! already knows how to negotiate whichever scheme (`$6$` = SHA-512,
//! `$y$` = yescrypt, ...) the stored hash's own prefix names.
//!
//! Privileges are dropped in the POSIX-mandated order --
//! `initgroups` before `setgid` before `setuid` -- because doing it
//! in any other order can leave the process holding root's
//! supplementary groups or gid even after `setuid` "succeeds": once
//! the effective uid is no longer 0, the earlier calls would no
//! longer be permitted to run at all. This is a well-documented class
//! of real privilege-escalation bug, not a theoretical one.

use crate::common::errors::{AppError, AppResult};
use std::ffi::CString;
use std::io;
use std::os::raw::{c_char, c_int};

#[cfg(unix)]
mod ffi {
    use super::*;

    // Layout matches glibc's `struct termios` on Linux (see
    // `<bits/termios.h>`): four `tcflag_t` (u32) fields, one `cc_t`
    // (u8) line-discipline byte, `NCCS` (32 on Linux) control-character
    // bytes, then two `speed_t` (u32) fields. `#[repr(C)]` inserts the
    // same padding before `c_ispeed` that C's own alignment rules
    // would, so this matches the real struct's size without needing
    // to spell the padding out by hand. Only `c_lflag` is ever read or
    // modified here -- the rest just round-trips unchanged.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Termios {
        pub c_iflag: u32,
        pub c_oflag: u32,
        pub c_cflag: u32,
        pub c_lflag: u32,
        pub c_line: u8,
        pub c_cc: [u8; 32],
        pub c_ispeed: u32,
        pub c_ospeed: u32,
    }

    extern "C" {
        pub fn crypt(key: *const c_char, salt: *const c_char) -> *mut c_char;
        pub fn setuid(uid: u32) -> c_int;
        pub fn setgid(gid: u32) -> c_int;
        pub fn initgroups(user: *const c_char, gid: u32) -> c_int;
        pub fn tcgetattr(fd: c_int, termios_p: *mut Termios) -> c_int;
        pub fn tcsetattr(fd: c_int, actions: c_int, termios_p: *const Termios) -> c_int;
    }
}

const ECHO: u32 = 0o0000010;
const ECHONL: u32 = 0o0000100;
const TCSAFLUSH: c_int = 2;

/// Read a line from `/dev/tty` with echo disabled, the same mechanism
/// (`tcgetattr`/clear `ECHO`+`ECHONL`/`tcsetattr`) every real password
/// prompt uses. Deliberately talks to `/dev/tty` rather than stdin:
/// that way the prompt still works (and still can't be silently fed
/// from a pipe) even when stdin has been redirected, and having no
/// controlling terminal at all is a hard error rather than a silent
/// fallback to an insecure path. Reads raw bytes and only decodes them
/// as UTF-8 once at the end, rather than casting byte-by-byte, so a
/// password containing non-ASCII characters round-trips correctly.
#[cfg(unix)]
pub fn prompt_password(prompt: &str) -> io::Result<String> {
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;

    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| io::Error::new(e.kind(), "no controlling terminal available"))?;
    let fd = tty.as_raw_fd();

    write!(tty, "{prompt}")?;
    tty.flush()?;

    let mut original: ffi::Termios = unsafe { std::mem::zeroed() };
    let got_attrs = unsafe { ffi::tcgetattr(fd, &mut original) } == 0;
    if got_attrs {
        let mut noecho = original;
        noecho.c_lflag &= !(ECHO | ECHONL);
        // Best-effort: if this fails the password may echo, which is
        // a UX problem, not a security one -- the actual boundary is
        // the crypt() comparison below, not whether typing was
        // visible -- so a failure here doesn't abort the prompt.
        unsafe { ffi::tcsetattr(fd, TCSAFLUSH, &noecho) };
    }

    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    let read_result: io::Result<()> = loop {
        match tty.read(&mut byte) {
            Ok(0) => break Ok(()),
            Ok(_) if byte[0] == b'\n' => break Ok(()),
            Ok(_) => bytes.push(byte[0]),
            Err(e) => break Err(e),
        }
    };

    if got_attrs {
        unsafe { ffi::tcsetattr(fd, TCSAFLUSH, &original) };
    }
    let _ = writeln!(tty); // the Enter keystroke itself wasn't echoed

    read_result?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(not(unix))]
pub fn prompt_password(_prompt: &str) -> io::Result<String> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "password prompts are only implemented on unix",
    ))
}

/// One `/etc/shadow` entry's password-hash field, or a reason it
/// can't be used to authenticate. Kept distinct from a bare
/// `Option<String>` so the locked/disabled cases are handled by name
/// instead of happening to fail whatever `crypt()` does with them.
enum ShadowHash {
    Hash(String),
    /// `!` or `!!`-prefixed: account is locked.
    Locked,
    /// `*` (or empty): password login is disabled for this account.
    Disabled,
}

fn classify_hash(field: &str) -> ShadowHash {
    if field.is_empty() || field == "*" {
        ShadowHash::Disabled
    } else if field.starts_with('!') {
        ShadowHash::Locked
    } else {
        ShadowHash::Hash(field.to_string())
    }
}

/// Look up `username`'s entry in `/etc/shadow`. Returns `Err` if the
/// file can't be read at all (almost always: this binary isn't
/// actually installed setuid-root, so the *caller* lacks permission --
/// a normal, expected condition when testing this without installing
/// it, not a corrupt-system condition), or `Ok(None)` if there's
/// simply no entry for `username`.
fn read_shadow(username: &str) -> io::Result<Option<ShadowHash>> {
    let contents = std::fs::read_to_string("/etc/shadow").map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "can't read /etc/shadow ({e}) -- is this binary installed setuid-root? \
                 it does nothing useful otherwise"
            ),
        )
    })?;
    for line in contents.lines() {
        let mut fields = line.split(':');
        let Some(name) = fields.next() else {
            continue;
        };
        if name != username {
            continue;
        }
        let hash_field = fields.next().unwrap_or("");
        return Ok(Some(classify_hash(hash_field)));
    }
    Ok(None)
}

/// Constant-time byte comparison for the two hash strings compared
/// below -- avoids a comparison that could return as soon as it finds
/// the first differing byte, which in principle a very precise local
/// timing measurement could use to narrow down a correct prefix.
/// Always walks the full length of both strings.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify `password` against `username`'s real `/etc/shadow` entry.
/// `Ok(false)` covers every "not this password" case uniformly (wrong
/// password, locked account, disabled account, no such user) so
/// callers can't accidentally branch differently on *why* it failed.
#[cfg(unix)]
pub fn verify_password(username: &str, password: &str) -> io::Result<bool> {
    let entry = match read_shadow(username)? {
        Some(entry) => entry,
        None => return Ok(false),
    };
    let stored = match entry {
        ShadowHash::Hash(h) => h,
        ShadowHash::Locked | ShadowHash::Disabled => return Ok(false),
    };

    let c_password =
        CString::new(password).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let c_stored = CString::new(stored.as_str())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let result_ptr = unsafe { ffi::crypt(c_password.as_ptr(), c_stored.as_ptr()) };
    if result_ptr.is_null() {
        return Ok(false);
    }
    let computed = unsafe { std::ffi::CStr::from_ptr(result_ptr) }
        .to_string_lossy()
        .into_owned();

    Ok(constant_time_eq(&computed, &stored))
}

#[cfg(not(unix))]
pub fn verify_password(_username: &str, _password: &str) -> io::Result<bool> {
    Ok(false)
}

/// Permanently drop from root to `(uid, gid)`, in the only order
/// that's actually safe -- see this module's doc comment. Returns an
/// error, and changes nothing further, the instant any step fails;
/// callers must treat that as fatal and never proceed to exec a shell
/// or command if this returns `Err`.
#[cfg(unix)]
pub fn drop_privileges_to(uid: u32, gid: u32, username: &str) -> io::Result<()> {
    let c_username =
        CString::new(username).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // initgroups populates the *supplementary* group list from
    // /etc/group for `username` -- must happen first, while still
    // privileged enough to do so.
    if unsafe { ffi::initgroups(c_username.as_ptr(), gid) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { ffi::setgid(gid) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { ffi::setuid(uid) } != 0 {
        return Err(io::Error::last_os_error());
    }

    // Belt-and-suspenders: confirm the drop actually stuck by reading
    // the identity back, rather than trusting a 0 return code alone.
    let identity = crate::common::users::current_identity();
    if identity.uid != uid || identity.euid != uid || identity.gid != gid || identity.egid != gid {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "privilege drop did not take effect as expected",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn drop_privileges_to(_uid: u32, _gid: u32, _username: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "privilege switching is only implemented on unix",
    ))
}

/// Shared "ask for a password up to 3 times, verifying each attempt"
/// loop used by both `su` and `sudo`.
pub fn authenticate_interactively(username: &str) -> AppResult<()> {
    for attempt in 1..=3 {
        let prompt = if attempt == 1 {
            "Password: ".to_string()
        } else {
            format!("Sorry, try again ({attempt}/3): ")
        };
        let password = prompt_password(&prompt)
            .map_err(|e| AppError::new(format!("cannot read password: {e}")))?;
        match verify_password(username, &password) {
            Ok(true) => return Ok(()),
            Ok(false) => continue,
            Err(e) => return Err(AppError::new(format!("cannot verify password: {e}"))),
        }
    }
    Err(AppError::with_code("Authentication failure", 1))
}
