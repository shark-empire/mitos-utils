//! Integration tests for the filesystem utilities: mkdir, touch,
//! ls, cat, cp, mv, rm, ln, basename, dirname, stat.
//!
//! Each test builds its own scratch directory under the system temp
//! dir (named with the process ID + a counter, so parallel `cargo
//! test` runs don't collide) and invokes the real compiled binaries
//! via `env!("CARGO_BIN_EXE_<name>")` -- no mocking, these run the
//! exact artifact that ships.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A scratch directory that removes itself when dropped.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("mitos-utils-test-{}-{}", std::process::id(), n));
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

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin).args(args).output().expect("spawn binary")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn run_with_stdin(bin: &str, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn binary");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait for output")
}

#[test]
fn mkdir_then_ls_shows_it() {
    let scratch = Scratch::new();
    let dir = scratch.join("child");

    let mkdir_out = run(env!("CARGO_BIN_EXE_mkdir"), &[dir.to_str().unwrap()]);
    assert!(mkdir_out.status.success());
    assert!(dir.is_dir());

    let ls_out = run(env!("CARGO_BIN_EXE_ls"), &[scratch.path.to_str().unwrap()]);
    assert!(ls_out.status.success());
    assert!(stdout_of(&ls_out).contains("child"));
}

#[test]
fn mkdir_without_parents_fails_on_missing_parent() {
    let scratch = Scratch::new();
    let nested = scratch.join("a/b/c");
    let out = run(env!("CARGO_BIN_EXE_mkdir"), &[nested.to_str().unwrap()]);
    assert!(!out.status.success());
}

#[test]
fn mkdir_dash_p_creates_parents() {
    let scratch = Scratch::new();
    let nested = scratch.join("a/b/c");
    let out = run(env!("CARGO_BIN_EXE_mkdir"), &["-p", nested.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(nested.is_dir());
}

#[test]
fn touch_creates_empty_file() {
    let scratch = Scratch::new();
    let file = scratch.join("new.txt");
    let out = run(env!("CARGO_BIN_EXE_touch"), &[file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(file.is_file());
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 0);
}

#[test]
fn cat_prints_file_contents() {
    let scratch = Scratch::new();
    let file = scratch.join("hello.txt");
    std::fs::write(&file, "hello mitos\n").unwrap();

    let out = run(env!("CARGO_BIN_EXE_cat"), &[file.to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "hello mitos\n");
}

#[test]
fn cat_reports_error_for_missing_file() {
    let scratch = Scratch::new();
    let missing = scratch.join("does-not-exist.txt");
    let out = run(env!("CARGO_BIN_EXE_cat"), &[missing.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(!out.stderr.is_empty());
}

#[test]
fn cp_copies_file_contents() {
    let scratch = Scratch::new();
    let src = scratch.join("src.txt");
    let dst = scratch.join("dst.txt");
    std::fs::write(&src, "copy me").unwrap();

    let out = run(env!("CARGO_BIN_EXE_cp"), &[src.to_str().unwrap(), dst.to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "copy me");
}

#[test]
fn cp_recursive_copies_directory_tree() {
    let scratch = Scratch::new();
    let src_dir = scratch.join("src_tree");
    std::fs::create_dir_all(src_dir.join("nested")).unwrap();
    std::fs::write(src_dir.join("nested/file.txt"), "deep").unwrap();
    let dst_dir = scratch.join("dst_tree");

    let out = run(
        env!("CARGO_BIN_EXE_cp"),
        &["-r", src_dir.to_str().unwrap(), dst_dir.to_str().unwrap()],
    );
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(dst_dir.join("nested/file.txt")).unwrap(), "deep");
}

#[test]
fn mv_renames_file() {
    let scratch = Scratch::new();
    let src = scratch.join("a.txt");
    let dst = scratch.join("b.txt");
    std::fs::write(&src, "move me").unwrap();

    let out = run(env!("CARGO_BIN_EXE_mv"), &[src.to_str().unwrap(), dst.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(!src.exists());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "move me");
}

#[test]
fn rm_removes_file() {
    let scratch = Scratch::new();
    let file = scratch.join("gone.txt");
    std::fs::write(&file, "x").unwrap();

    let out = run(env!("CARGO_BIN_EXE_rm"), &[file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(!file.exists());
}

#[test]
fn rm_without_recursive_refuses_directory() {
    let scratch = Scratch::new();
    let dir = scratch.join("adir");
    std::fs::create_dir(&dir).unwrap();

    let out = run(env!("CARGO_BIN_EXE_rm"), &[dir.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(dir.exists());
}

#[test]
fn rm_recursive_removes_directory_tree() {
    let scratch = Scratch::new();
    let dir = scratch.join("adir");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/f.txt"), "x").unwrap();

    let out = run(env!("CARGO_BIN_EXE_rm"), &["-r", dir.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(!dir.exists());
}

#[test]
fn rm_force_ignores_missing_file() {
    let scratch = Scratch::new();
    let missing = scratch.join("nope.txt");
    let out = run(env!("CARGO_BIN_EXE_rm"), &["-f", missing.to_str().unwrap()]);
    assert!(out.status.success());
}

#[test]
fn ln_symbolic_creates_readable_link() {
    let scratch = Scratch::new();
    let target = scratch.join("target.txt");
    let link = scratch.join("link.txt");
    std::fs::write(&target, "linked content").unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_ln"),
        &["-s", target.to_str().unwrap(), link.to_str().unwrap()],
    );
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "linked content");
}

#[test]
fn pwd_prints_current_directory() {
    let out = run(env!("CARGO_BIN_EXE_pwd"), &[]);
    assert!(out.status.success());
    assert!(!stdout_of(&out).trim().is_empty());
}

#[test]
fn stat_reports_missing_file() {
    let scratch = Scratch::new();
    let missing = scratch.join("nope.txt");
    let out = run(env!("CARGO_BIN_EXE_stat"), &[missing.to_str().unwrap()]);
    assert!(!out.status.success());
}

#[test]
fn stat_reports_existing_file_size() {
    let scratch = Scratch::new();
    let file = scratch.join("sized.txt");
    std::fs::write(&file, "12345").unwrap();

    let out = run(env!("CARGO_BIN_EXE_stat"), &[file.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout_of(&out).contains('5'));
}

#[test]
fn rm_interactive_declines_on_no() {
    let scratch = Scratch::new();
    let file = scratch.join("keep.txt");
    std::fs::write(&file, "x").unwrap();

    let out = run_with_stdin(env!("CARGO_BIN_EXE_rm"), &["-i", file.to_str().unwrap()], "n\n");
    assert!(out.status.success());
    assert!(file.exists(), "file should survive a declined -i prompt");
}

#[test]
fn rm_interactive_removes_on_yes() {
    let scratch = Scratch::new();
    let file = scratch.join("gone.txt");
    std::fs::write(&file, "x").unwrap();

    let out = run_with_stdin(env!("CARGO_BIN_EXE_rm"), &["-i", file.to_str().unwrap()], "y\n");
    assert!(out.status.success());
    assert!(!file.exists(), "file should be removed after a confirmed -i prompt");
}

#[test]
fn cp_interactive_declines_overwrite_on_no() {
    let scratch = Scratch::new();
    let src = scratch.join("src.txt");
    let dst = scratch.join("dst.txt");
    std::fs::write(&src, "new content").unwrap();
    std::fs::write(&dst, "original content").unwrap();

    let out = run_with_stdin(
        env!("CARGO_BIN_EXE_cp"),
        &["-i", src.to_str().unwrap(), dst.to_str().unwrap()],
        "n\n",
    );
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "original content");
}

#[test]
fn cp_interactive_overwrites_on_yes() {
    let scratch = Scratch::new();
    let src = scratch.join("src.txt");
    let dst = scratch.join("dst.txt");
    std::fs::write(&src, "new content").unwrap();
    std::fs::write(&dst, "original content").unwrap();

    let out = run_with_stdin(
        env!("CARGO_BIN_EXE_cp"),
        &["-i", src.to_str().unwrap(), dst.to_str().unwrap()],
        "y\n",
    );
    assert!(out.status.success());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "new content");
}

#[test]
fn cp_dash_p_preserves_modification_time() {
    let scratch = Scratch::new();
    let src = scratch.join("src.txt");
    let dst = scratch.join("dst.txt");
    std::fs::write(&src, "x").unwrap();

    // Backdate the source's mtime so a copy that *doesn't* preserve
    // it (the non -p default) would produce a detectably different
    // timestamp on the copy.
    let backdated = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&src)
        .unwrap()
        .set_modified(backdated)
        .unwrap();

    let out = run(env!("CARGO_BIN_EXE_cp"), &["-p", src.to_str().unwrap(), dst.to_str().unwrap()]);
    assert!(out.status.success());

    let src_mtime = std::fs::metadata(&src).unwrap().modified().unwrap();
    let dst_mtime = std::fs::metadata(&dst).unwrap().modified().unwrap();
    assert_eq!(src_mtime, dst_mtime);
}
