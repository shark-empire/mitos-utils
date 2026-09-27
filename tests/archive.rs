//! Integration tests for `tar`: create/extract/list round-trips
//! against the compiled binaries, plus a hand-built archive (built
//! independently of `src/applets/tar.rs`'s own header-writing code,
//! so this isn't just checking the implementation against itself) to
//! confirm the `..`-path-traversal extraction guard described in
//! tar.rs's own SECURITY doc comment actually holds.
//!
//! `useradd`/`groupadd`/`passwd`/`usermod` have no equivalent
//! integration tests here, deliberately, same as before: they need
//! to run as root against real `/etc/passwd`/`/etc/shadow`/`/etc/group`,
//! which isn't something to do to whatever account happens to run
//! the test suite.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("mitos-utils-archive-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Scratch { path }
    }
    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn tar_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tar")
}

fn run(args: &[&str]) -> Output {
    Command::new(tar_bin()).args(args).output().expect("spawn tar")
}

#[test]
fn create_then_extract_round_trips_files_dirs_and_symlinks() {
    let src = Scratch::new();
    let dest = Scratch::new();

    std::fs::write(src.join("hello.txt"), b"hello, mitos").unwrap();
    std::fs::create_dir_all(src.join("subdir")).unwrap();
    std::fs::write(src.join("subdir/nested.txt"), b"nested contents").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("hello.txt", src.join("link-to-hello")).unwrap();

    let archive = src.join("out.tar");
    let create = Command::new(tar_bin())
        .current_dir(&src.path)
        .args([
            "-c",
            "-f",
            archive.to_str().unwrap(),
            "hello.txt",
            "subdir",
            "link-to-hello",
        ])
        .output()
        .expect("spawn tar -c");
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );

    let extract = run(&[
        "-x",
        "-f",
        archive.to_str().unwrap(),
        "-C",
        dest.path.to_str().unwrap(),
    ]);
    assert!(
        extract.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&extract.stderr)
    );

    assert_eq!(std::fs::read(dest.join("hello.txt")).unwrap(), b"hello, mitos");
    assert_eq!(
        std::fs::read(dest.join("subdir/nested.txt")).unwrap(),
        b"nested contents"
    );
    #[cfg(unix)]
    {
        let target = std::fs::read_link(dest.join("link-to-hello")).unwrap();
        assert_eq!(target, PathBuf::from("hello.txt"));
    }
}

#[test]
fn list_prints_member_names() {
    let src = Scratch::new();
    std::fs::write(src.join("a.txt"), b"aaa").unwrap();
    std::fs::write(src.join("b.txt"), b"bbb").unwrap();
    let archive = src.join("list.tar");

    let create = Command::new(tar_bin())
        .current_dir(&src.path)
        .args(["-c", "-f", archive.to_str().unwrap(), "a.txt", "b.txt"])
        .output()
        .expect("spawn tar -c");
    assert!(create.status.success());

    let list = run(&["-t", "-f", archive.to_str().unwrap()]);
    assert!(list.status.success());
    let out = String::from_utf8_lossy(&list.stdout);
    assert!(out.contains("a.txt"));
    assert!(out.contains("b.txt"));
}

#[test]
fn bundled_letters_form_works_like_explicit_flags() {
    let src = Scratch::new();
    std::fs::write(src.join("f.txt"), b"bundled").unwrap();
    let archive = src.join("bundled.tar");

    // Classic `tar cf archive file` -- no leading dash on the mode
    // letters -- should behave exactly like `-c -f archive file`.
    let create = Command::new(tar_bin())
        .current_dir(&src.path)
        .args(["cf", archive.to_str().unwrap(), "f.txt"])
        .output()
        .expect("spawn bundled tar cf");
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    assert!(archive.exists());

    let list = run(&["-t", "-f", archive.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&list.stdout).contains("f.txt"));
}

#[test]
fn compression_flags_are_refused_with_a_clear_message() {
    let out = run(&["-c", "-z", "-f", "whatever.tar.gz", "somefile"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(stderr.contains("not supported") || stderr.contains("compress"));
}

/// Builds one ustar 512-byte header block (POSIX.1-1988 layout) plus
/// its data, by hand, independently of `tar.rs`'s own header-writing
/// code -- this test should catch a bug in either side, not just
/// confirm the implementation agrees with itself.
fn octal_field(value: u64, field_width: usize) -> Vec<u8> {
    let digits = field_width - 1;
    let mut bytes = format!("{:0digits$o}", value, digits = digits).into_bytes();
    bytes.push(0);
    bytes
}

fn build_archive_with_name(name: &[u8]) -> Vec<u8> {
    let content = b"pwned";
    let mut block = [0u8; 512];
    block[0..name.len()].copy_from_slice(name);
    block[100..108].copy_from_slice(&octal_field(0o644, 8)); // mode
    block[108..116].copy_from_slice(&octal_field(0, 8)); // uid
    block[116..124].copy_from_slice(&octal_field(0, 8)); // gid
    block[124..136].copy_from_slice(&octal_field(content.len() as u64, 12)); // size
    block[136..148].copy_from_slice(&octal_field(0, 12)); // mtime
    block[156] = b'0'; // typeflag: regular file
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");

    let mut sum: u32 = 0;
    for (i, &b) in block.iter().enumerate() {
        if (148..156).contains(&i) {
            sum += b' ' as u32;
        } else {
            sum += b as u32;
        }
    }
    let chksum = format!("{:06o}\0 ", sum).into_bytes();
    block[148..156].copy_from_slice(&chksum);

    let mut out = block.to_vec();
    out.extend_from_slice(content);
    let pad = (512 - (content.len() % 512)) % 512;
    out.extend(std::iter::repeat(0u8).take(pad));
    out.extend(std::iter::repeat(0u8).take(512 * 2)); // end-of-archive marker
    out
}

#[test]
fn extraction_refuses_a_dotdot_path_traversal_entry() {
    let scratch = Scratch::new();
    let archive = scratch.join("evil.tar");
    std::fs::write(&archive, build_archive_with_name(b"../evil.txt")).unwrap();

    let dest = scratch.join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let extract = run(&[
        "-x",
        "-f",
        archive.to_str().unwrap(),
        "-C",
        dest.to_str().unwrap(),
    ]);
    // The one bad entry makes the overall command report failure...
    assert!(!extract.status.success());
    assert!(String::from_utf8_lossy(&extract.stderr).contains(".."));

    // ...and, the property that actually matters: nothing landed
    // outside `dest`. A naive `dest.join("../evil.txt")` would have
    // written one directory up from `dest` -- check exactly there,
    // plus the two other plausible spots.
    assert!(!dest.join("evil.txt").exists());
    assert!(!scratch.join("evil.txt").exists());
    assert!(!dest.parent().unwrap().join("evil.txt").exists());
}

#[test]
fn extraction_strips_a_leading_slash_instead_of_writing_absolute() {
    let scratch = Scratch::new();
    let archive = scratch.join("absolute.tar");
    std::fs::write(&archive, build_archive_with_name(b"/etc/evil.txt")).unwrap();

    let dest = scratch.join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let extract = run(&[
        "-x",
        "-f",
        archive.to_str().unwrap(),
        "-C",
        dest.to_str().unwrap(),
    ]);
    assert!(
        extract.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&extract.stderr)
    );
    // The leading '/' is stripped, so this lands *inside* dest as a
    // relative "etc/evil.txt", never at the real /etc/evil.txt.
    assert_eq!(std::fs::read(dest.join("etc/evil.txt")).unwrap(), b"pwned");
    assert!(!std::path::Path::new("/etc/evil.txt").exists());
}
