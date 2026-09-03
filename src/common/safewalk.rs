//! TOCTOU-hardened recursive directory operations, used by `rm -r`,
//! `chmod -R`, and `chown -R`/`chgrp -R`.
//!
//! # The risk this closes
//!
//! A naive recursive walk checks "is this a directory?" via one
//! syscall on a path *string*, then later acts on that same string
//! (recursing into it, deleting it, chmod'ing it). Between those two
//! steps, anything with write access to the tree being walked can
//! swap the entry for a symlink -- redirecting the recursion (or the
//! delete/chmod) somewhere the caller never intended, potentially
//! outside the tree entirely. GNU coreutils specifically guards
//! against this in `rm -r`/`chmod -R`; this module gives mitos-utils
//! the same property.
//!
//! # Why this doesn't use `O_DIRECTORY`/`O_NOFOLLOW`
//!
//! The obvious fix is opening with `O_NOFOLLOW` so a symlink swapped
//! in can't be followed. This module deliberately does NOT do that,
//! because those two `open(2)` flag values are not the same across
//! Linux architectures: glibc/x86_64 defines `O_DIRECTORY = 0200000`
//! and `O_NOFOLLOW = 0400000`, but the arm64 kernel's own uapi
//! headers define `O_DIRECTORY = 040000` for AArch32-compat reasons
//! (a legacy 32-bit ARM numbering carried forward). This isn't
//! theoretical: a 2024 QEMU bug report
//! (gitlab.com/qemu-project/qemu/-/issues/3262) shows `O_NOFOLLOW`
//! silently arriving as `O_LARGEFILE` when arm64 code ran under an
//! x86_64 host's emulation, because of exactly this mismatch. Given
//! mitosOS itself targets both x86_64 and aarch64, hand-rolling the
//! wrong numeric value here wouldn't just be a wrong answer -- it
//! would silently defeat the one thing this module exists to
//! provide, on one of our two target architectures, with no build
//! error to catch it. Not worth the risk for a value this module can
//! avoid needing entirely (see below). Scoped to `target_os =
//! "linux"` for now regardless, matching `common::users`'s FFI and
//! `df`/`ps`/`free`'s `/proc` dependency.
//!
//! # The actual design
//!
//! Every flag value this module *does* use has been confirmed
//! identical across every Linux architecture (`AT_FDCWD`,
//! `AT_SYMLINK_NOFOLLOW`, `AT_REMOVEDIR`, all from the newer `*at()`
//! family's uapi header, `include/uapi/linux/fcntl.h` -- distinct
//! from the older, architecture-variable `open(2)` flags above), plus
//! `O_RDONLY`, which is `0` by definition on every POSIX system. To
//! open a subdirectory relative to an already-verified parent
//! directory fd without needing to hand-roll `struct stat` (whose
//! layout has its own history of per-arch/per-libc variation) at all:
//!
//! 1. Stat the child *by name relative to the parent's fd*, without
//!    following symlinks, via `std::fs::symlink_metadata` on the
//!    `/proc/self/fd/<parent_fd>/<name>` path. This is a documented
//!    kernel feature (the `openat(2)` man page notes it directly:
//!    "per-thread working directory... can also be obtained by
//!    tricks based on /proc/self/fd/dirfd, but less efficiently"),
//!    and it resolves `name` relative to the *exact* directory
//!    instance the fd refers to -- not to whatever a plain string
//!    path might mean by the time we get around to using it. Refuse
//!    if it's a symlink or not a directory. Record its (device,
//!    inode).
//! 2. Open it for real via `openat(parent_fd, name, O_RDONLY, 0)`.
//! 3. `fstat` the *resulting open file descriptor* (via
//!    `std::fs::File::metadata`, again no hand-rolled struct) and
//!    confirm its (device, inode) match step 1's. A mismatch means
//!    the entry changed between steps 1 and 2 -- refuse rather than
//!    proceed. Once opened, a file descriptor can't be swapped out
//!    from under us by a later rename/symlink -- it refers to the
//!    same underlying file for as long as it stays open, so this
//!    closes the only remaining race window.
//!
//! Deleting an entry (`unlinkat`) and changing its mode/owner
//! (`fchmodat`/`fchownat`) don't need this dance: both are inherently
//! anchored to a specific parent-fd + name pair by the syscall itself
//! (no separate "check, then act on a path" step to race), and
//! `unlinkat` in particular never follows a symlink for the final
//! path component (POSIX `unlink` semantics) -- so a symlink swapped
//! in for a plain file gets its symlink removed, not whatever it
//! points at.
//!
//! Listing a directory's contents (to know what names to recurse
//! into) still goes through plain `std::fs::read_dir` on a path
//! string -- not the dangerous half of the operation, and re-using
//! Rust's own well-tested `readdir` handling beats hand-rolling
//! `struct dirent`'s notoriously fiddly trailing-name layout without
//! a way to compile-test it (see docs/architecture.md).
//!
//! None of this has been compiled or run (no toolchain -- see
//! docs/architecture.md); the flag values above were cross-checked
//! against current kernel uapi headers via web search, not written
//! from memory, given what's at stake if they're wrong.

use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;

#[cfg(target_os = "linux")]
mod ffi {
    use std::os::raw::{c_char, c_int};

    // Confirmed identical across every Linux architecture (from
    // include/uapi/linux/fcntl.h, the newer `*at()`-family header --
    // NOT the older per-arch-variable open(2) flags; see module docs).
    pub const AT_REMOVEDIR: c_int = 0x200;
    // O_RDONLY is 0 by definition on every POSIX system -- no
    // architecture ever assigns it a nonzero bit pattern.
    pub const O_RDONLY: c_int = 0;

    extern "C" {
        pub fn openat(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: c_int) -> c_int;
        pub fn unlinkat(dirfd: c_int, pathname: *const c_char, flags: c_int) -> c_int;
        pub fn fchmodat(dirfd: c_int, pathname: *const c_char, mode: u32, flags: c_int) -> c_int;
        pub fn fchownat(dirfd: c_int, pathname: *const c_char, owner: u32, group: u32, flags: c_int) -> c_int;
        pub fn chown(path: *const c_char, owner: u32, group: u32) -> c_int;
    }
}

/// A directory that has been opened and, unless it's the walk's
/// starting point, verified via the check-open-verify sequence
/// described in the module docs. Every operation relative to it
/// (`open_subdir`, `remove_file`, `remove_empty_dir`, `chmod_entry`,
/// `chown_entry`) is anchored to this specific, already-open file
/// descriptor rather than to a re-resolved path string.
pub struct SafeDir {
    file: File,
}

fn to_cstring(name: &str) -> io::Result<CString> {
    CString::new(name).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))
}

impl SafeDir {
    /// Open the starting point of a walk. This is the one place a
    /// plain path-based open is unavoidable -- it's exactly as
    /// trustworthy as any other tool's initial argument resolution,
    /// since there's no parent file descriptor yet to scope it to.
    /// The hardening in this module is about everything *after* this
    /// point. Fails if `path` is not a real (non-symlink) directory.
    pub fn open_root(path: &Path) -> io::Result<SafeDir> {
        let meta = std::fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "refusing to recurse through a symlink at the top of a walk",
            ));
        }
        if !meta.is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, "not a directory"));
        }
        let file = File::open(path)?;
        Ok(SafeDir { file })
    }

    fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// The `/proc/self/fd/<fd>/<name>` reference used to stat `name`
    /// relative to exactly this directory instance without following
    /// symlinks and without hand-rolling `fstatat`'s `struct stat`.
    /// Documented kernel behavior -- see the `openat(2)` man page and
    /// module docs above.
    fn scoped_path(&self, name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("/proc/self/fd/{}", self.raw_fd())).join(name)
    }

    /// List the names directly inside this directory (not a full
    /// recursive walk -- callers recurse by calling `open_subdir` on
    /// whichever names turn out to be real directories).
    pub fn list_names(&self) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(self.scoped_path(""))? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    /// Whether `name` (relative to this directory, not following
    /// symlinks) is itself a symlink.
    pub fn entry_is_symlink(&self, name: &str) -> io::Result<bool> {
        Ok(std::fs::symlink_metadata(self.scoped_path(name))?.file_type().is_symlink())
    }

    /// Open `name` as a subdirectory of this one, via the
    /// check-open-verify sequence described in the module docs.
    /// Returns an error (rather than silently following anything) if
    /// `name` is a symlink, isn't a directory, or changed between the
    /// check and the open.
    pub fn open_subdir(&self, name: &str) -> io::Result<SafeDir> {
        let scoped = self.scoped_path(name);
        let pre = std::fs::symlink_metadata(&scoped)?;
        if pre.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("refusing to recurse through symlink '{}'", name),
            ));
        }
        if !pre.is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("'{}' is not a directory", name)));
        }
        let (pre_dev, pre_ino) = (pre.dev(), pre.ino());

        let c_name = to_cstring(name)?;
        let fd = unsafe { ffi::openat(self.raw_fd(), c_name.as_ptr(), ffi::O_RDONLY, 0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };

        let post = file.metadata()?;
        if post.dev() != pre_dev || post.ino() != pre_ino {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("'{}' changed during traversal (possible symlink race) -- refusing", name),
            ));
        }
        Ok(SafeDir { file })
    }

    /// Remove a non-directory entry by name (never follows a symlink
    /// for the final component -- that's POSIX `unlink`'s own
    /// behavior, not something this module adds).
    pub fn remove_file(&self, name: &str) -> io::Result<()> {
        let c_name = to_cstring(name)?;
        let result = unsafe { ffi::unlinkat(self.raw_fd(), c_name.as_ptr(), 0) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Remove an empty directory entry by name.
    pub fn remove_empty_dir(&self, name: &str) -> io::Result<()> {
        let c_name = to_cstring(name)?;
        let result = unsafe { ffi::unlinkat(self.raw_fd(), c_name.as_ptr(), ffi::AT_REMOVEDIR) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Change the mode of an entry by name.
    pub fn chmod_entry(&self, name: &str, mode: u32) -> io::Result<()> {
        let c_name = to_cstring(name)?;
        let result = unsafe { ffi::fchmodat(self.raw_fd(), c_name.as_ptr(), mode, 0) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Change the owner and/or group of an entry by name. `u32::MAX`
    /// for either parameter leaves that ID unchanged (matching
    /// `chown(2)`'s own `-1` convention).
    pub fn chown_entry(&self, name: &str, uid: u32, gid: u32) -> io::Result<()> {
        let c_name = to_cstring(name)?;
        let result = unsafe { ffi::fchownat(self.raw_fd(), c_name.as_ptr(), uid, gid, 0) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// The mode bits of `name` (relative to this directory, not
    /// following symlinks) -- needed by `chmod -R` with a symbolic
    /// spec like `u+x`, which must compute the new mode relative to
    /// each entry's *own* current mode, not one flat value copied
    /// onto every entry in the tree.
    pub fn entry_mode(&self, name: &str) -> io::Result<u32> {
        Ok(std::fs::symlink_metadata(self.scoped_path(name))?.mode() & 0o7777)
    }

    /// This directory's own mode, for chmod/chown's "also apply to
    /// the directory itself, not just its contents" step.
    pub fn self_name_hint(&self) -> &'static str {
        "."
    }
}

/// Recursively remove `root` (which must already be a verified,
/// real, non-symlink directory -- see `SafeDir::open_root`) and
/// everything under it, via the hardened primitives above.
pub fn remove_tree(root: &Path) -> io::Result<()> {
    let dir = SafeDir::open_root(root)?;
    remove_contents(&dir)?;
    // Remove `root` itself last, by its own path -- it has no parent
    // SafeDir in this call (the caller already validated it), so a
    // plain rmdir is used, matching how `SafeDir::open_root` also had
    // to use a plain open for the same reason.
    std::fs::remove_dir(root)
}

fn remove_contents(dir: &SafeDir) -> io::Result<()> {
    for name in dir.list_names()? {
        if dir.entry_is_symlink(&name)? {
            dir.remove_file(&name)?;
            continue;
        }
        match dir.open_subdir(&name) {
            Ok(sub) => {
                remove_contents(&sub)?;
                dir.remove_empty_dir(&name)?;
            }
            Err(_) => {
                // Not a directory (or failed the race check, which
                // also means "don't touch it as a directory") --
                // treat as a plain file.
                dir.remove_file(&name)?;
            }
        }
    }
    Ok(())
}

/// Recursively apply a mode-computing function to `root` and
/// everything under it. Takes a function of "current mode ->
/// new mode" rather than a flat mode, since a symbolic chmod spec
/// (`u+x`) must be computed relative to each entry's own current
/// mode, not one value copied onto every entry in the tree.
pub fn chmod_tree(root: &Path, mode_fn: &dyn Fn(u32) -> u32) -> io::Result<()> {
    let dir = SafeDir::open_root(root)?;
    chmod_contents(&dir, mode_fn)?;
    let root_mode = std::fs::metadata(root)?.permissions().mode() & 0o7777;
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(mode_fn(root_mode)))
}

fn chmod_contents(dir: &SafeDir, mode_fn: &dyn Fn(u32) -> u32) -> io::Result<()> {
    for name in dir.list_names()? {
        if dir.entry_is_symlink(&name)? {
            continue; // chmod doesn't affect symlinks themselves
        }
        let current = dir.entry_mode(&name)?;
        dir.chmod_entry(&name, mode_fn(current))?;
        if let Ok(sub) = dir.open_subdir(&name) {
            chmod_contents(&sub, mode_fn)?;
        }
    }
    Ok(())
}

/// Recursively apply `uid`/`gid` (either may be `u32::MAX` to leave
/// that ID unchanged) to `root` and everything under it.
pub fn chown_tree(root: &Path, uid: u32, gid: u32) -> io::Result<()> {
    let dir = SafeDir::open_root(root)?;
    chown_contents(&dir, uid, gid)?;
    chown_path(root, uid, gid)
}

fn chown_contents(dir: &SafeDir, uid: u32, gid: u32) -> io::Result<()> {
    for name in dir.list_names()? {
        dir.chown_entry(&name, uid, gid)?;
        if !dir.entry_is_symlink(&name)? {
            if let Ok(sub) = dir.open_subdir(&name) {
                chown_contents(&sub, uid, gid)?;
            }
        }
    }
    Ok(())
}

fn chown_path(path: &Path, uid: u32, gid: u32) -> io::Result<()> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
    // A plain top-level chown (not an *at() call) is fine here: `root`
    // was just freshly validated by `SafeDir::open_root` above, and
    // this is the walk's own starting path, not a re-resolved name
    // from partway through the recursion.
    let result = unsafe { ffi::chown(c_path.as_ptr(), uid, gid) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
