//! Shared primitives for `useradd`/`groupadd`/`passwd`: locking,
//! atomic rewrites, and allocating a free uid/gid.
//!
//! # Security status -- read this before relying on it
//!
//! Same weight as `common::auth` (read that module's doc comment
//! too) -- **not audited, not run on a real system**. This mutates
//! `/etc/passwd`, `/etc/shadow`, and `/etc/group` directly; a bug
//! here can corrupt the account database for the whole system, not
//! just misbehave for one user. Needs real review and disposable-VM
//! testing before trusting it anywhere real, same as `su`/`sudo`.
//!
//! Locking uses `/etc/.pwd.lock` -- the same lock file real
//! shadow-utils uses, taken the same way real shadow-utils takes it:
//! an `fcntl()` `F_WRLCK` record lock (via `F_SETLKW`), not a
//! `flock()` lock. Those are two different, non-interoperable
//! kernel-level mechanisms -- a `flock()` never blocks on, or is
//! blocked by, an `fcntl()` lock on the same file, even though both
//! nominally "lock the same path". Getting that mechanism wrong
//! would have meant this lock doing nothing at all against a real
//! `useradd`/`passwd` running at the same time, despite agreeing on
//! the file to lock -- confirmed against glibc's actual `lckpwdf()`
//! source (`shadow/lckpwdf.c`) before writing this, not assumed.

use std::io::Write;
use std::os::raw::c_int;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const LOCK_PATH: &str = "/etc/.pwd.lock";
const MIN_UID: u32 = 1000;
const MIN_GID: u32 = 1000;

mod ffi {
    use super::c_int;

    // Layout matches glibc's `struct flock` on 64-bit Linux (x86_64
    // and aarch64 both use a 64-bit off_t): two `short`s, then
    // `off_t` start/len -- `#[repr(C)]` inserts the same 4 bytes of
    // padding after `l_whence` that C's own alignment rules would,
    // to get `l_start` onto an 8-byte boundary -- then a `pid_t`.
    // `sizeof(struct flock) == 32` on both target architectures.
    #[repr(C)]
    pub struct Flock {
        pub l_type: i16,
        pub l_whence: i16,
        pub l_start: i64,
        pub l_len: i64,
        pub l_pid: i32,
    }

    extern "C" {
        pub fn fcntl(fd: c_int, cmd: c_int, lock: *mut Flock) -> c_int;
    }
}

const F_SETLK: c_int = 6;
const F_SETLKW: c_int = 7;
const F_WRLCK: i16 = 1;
const F_UNLCK: i16 = 2;
const SEEK_SET: i16 = 0;

fn whole_file_lock(lock_type: i16) -> ffi::Flock {
    ffi::Flock {
        l_type: lock_type,
        l_whence: SEEK_SET,
        l_start: 0,
        l_len: 0, // 0 means "to end of file", i.e. the whole file
        l_pid: 0, // only meaningful for F_GETLK's result, not SETLK(W)
    }
}

/// Holds `/etc/.pwd.lock` for as long as it's alive, releasing it on
/// drop (both explicitly, and implicitly when the held file's own
/// `Drop` closes its fd -- the explicit unlock is belt-and-suspenders,
/// not load-bearing).
pub struct AccountsLock {
    file: std::fs::File,
}

impl AccountsLock {
    pub fn acquire() -> std::io::Result<Self> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(LOCK_PATH)?;
        let mut lock = whole_file_lock(F_WRLCK);
        // F_SETLKW: blocks until the lock is available, rather than
        // failing immediately the way F_SETLK would -- useradd/passwd
        // are short-lived interactive commands, so waiting briefly
        // for a concurrent one to finish is the right default.
        if unsafe { ffi::fcntl(file.as_raw_fd(), F_SETLKW, &mut lock) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { file })
    }
}

impl Drop for AccountsLock {
    fn drop(&mut self) {
        use std::os::unix::io::AsRawFd;
        let mut lock = whole_file_lock(F_UNLCK);
        unsafe { ffi::fcntl(self.file.as_raw_fd(), F_SETLK, &mut lock) };
    }
}

/// Overwrite `path` with `content`, atomically: written to a
/// same-directory temp file first (so the rename below stays on one
/// filesystem, which is what makes it atomic), with `mode` set
/// explicitly rather than left to whatever the process umask would
/// otherwise filter it down to, *then* renamed over the original --
/// so a crash or a concurrent reader never sees a half-written file.
pub fn atomic_rewrite(path: &str, content: &str, mode: u32) -> std::io::Result<()> {
    let tmp_path = format!("{path}.tmp.{}", std::process::id());
    {
        let mut tmp = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(&tmp_path)?;
        tmp.write_all(content.as_bytes())?;
        tmp.sync_all()?;
    }
    // Explicit chmod even though `mode` was already passed to
    // `open()` above: that mode is filtered through the process
    // umask, so it isn't guaranteed to be exactly `mode` without this.
    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(mode))?;
    std::fs::rename(&tmp_path, path)
}

/// The next unused id at or above `min_id` in `entries_file`'s
/// `id_field_index`'th colon-separated field (2 for both
/// `/etc/passwd`'s uid and `/etc/group`'s gid).
fn next_free_id(entries_file: &str, min_id: u32, id_field_index: usize) -> std::io::Result<u32> {
    let contents = std::fs::read_to_string(entries_file)?;
    let max_existing = contents
        .lines()
        .filter_map(|line| line.split(':').nth(id_field_index))
        .filter_map(|s| s.parse::<u32>().ok())
        .filter(|&id| id >= min_id)
        .max();
    Ok(max_existing.map_or(min_id, |m| m + 1))
}

pub fn next_free_uid() -> std::io::Result<u32> {
    next_free_id("/etc/passwd", MIN_UID, 2)
}

pub fn next_free_gid() -> std::io::Result<u32> {
    next_free_id("/etc/group", MIN_GID, 2)
}

/// The POSIX "portable filename character set" username rule real
/// `useradd` enforces by default: starts with a letter or
/// underscore, then only letters/digits/underscore/hyphen, 32
/// characters max.
pub fn validate_username(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 32 {
        return Err("name must be 1-32 characters".to_string());
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err("name must start with a letter or underscore".to_string());
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("name may only contain letters, digits, '_', and '-'".to_string());
    }
    Ok(())
}

pub fn username_exists(name: &str) -> bool {
    std::fs::read_to_string("/etc/passwd")
        .map(|c| c.lines().any(|l| l.split(':').next() == Some(name)))
        .unwrap_or(false)
}

pub fn groupname_exists(name: &str) -> bool {
    std::fs::read_to_string("/etc/group")
        .map(|c| c.lines().any(|l| l.split(':').next() == Some(name)))
        .unwrap_or(false)
}
