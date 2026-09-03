//! Integration tests for the cross-cutting CLI infrastructure added
//! after the initial build-out: centralized `--help`/`--version`
//! (`common::errors::run`), POSIX `--` end-of-options handling
//! (`common::args::split_dashdash`), and the `mitos-box` multiplexed
//! dispatcher.

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
        let path = std::env::temp_dir().join(format!("mitos-utils-cliinfra-{}-{}", std::process::id(), n));
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

#[test]
fn help_prints_usage_and_exits_zero_without_running_the_tool() {
    // `--help` on `mkdir` with no other operand would normally be a
    // "missing operand" usage error -- if this instead succeeds and
    // prints a usage line, --help correctly short-circuited before
    // mkdir's own argument handling ever ran.
    let out = run(env!("CARGO_BIN_EXE_mkdir"), &["--help"]);
    assert!(out.status.success());
    assert!(stdout_of(&out).to_lowercase().contains("usage"));
}

#[test]
fn version_prints_something_and_exits_zero() {
    let out = run(env!("CARGO_BIN_EXE_cat"), &["--version"]);
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains("cat"));
    assert!(!text.trim().is_empty());
}

#[test]
fn help_does_not_shadow_ls_dash_h_human_readable() {
    // `-h` must keep meaning "human-readable" for ls/du, not become
    // a help shorthand -- see common::errors::run's doc comment.
    let scratch = Scratch::new();
    let out = run(env!("CARGO_BIN_EXE_ls"), &["-l", "-h", scratch.path.to_str().unwrap()]);
    assert!(out.status.success());
    // A real listing (even of an empty dir) succeeds silently; if -h
    // had been hijacked as --help this would instead print a "usage:
    // ls ..." line instead of doing an ls.
    assert!(!stdout_of(&out).to_lowercase().contains("usage:"));
}

#[test]
fn double_dash_allows_operating_on_a_dash_prefixed_filename() {
    let scratch = Scratch::new();
    let weird = scratch.join("-oddly-named-file.txt");
    std::fs::write(&weird, "content").unwrap();

    let out = run(env!("CARGO_BIN_EXE_cat"), &["--", weird.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout_of(&out), "content\n");
}

#[test]
fn double_dash_lets_rm_remove_a_dash_prefixed_filename() {
    let scratch = Scratch::new();
    let weird = scratch.join("-weird.txt");
    std::fs::write(&weird, "x").unwrap();

    let out = run(env!("CARGO_BIN_EXE_rm"), &["--", weird.to_str().unwrap()]);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(!weird.exists());
}

#[test]
fn mitos_box_dispatches_by_explicit_applet_name() {
    let out = run(env!("CARGO_BIN_EXE_mitos-box"), &["echo", "hello", "box"]);
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "hello box\n");
}

#[test]
fn mitos_box_unknown_applet_lists_available_ones() {
    let out = run(env!("CARGO_BIN_EXE_mitos-box"), &["not-a-real-applet"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown applet"));
    assert!(stderr.contains("cat")); // one of the applets in the listing
}

#[test]
fn mitos_box_with_no_arguments_lists_applets_and_exits_zero() {
    let out = run(env!("CARGO_BIN_EXE_mitos-box"), &[]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("available applets"));
}

#[test]
fn mitos_box_help_and_direct_binary_agree_on_usage_text() {
    // The whole point of routing mitos-box through the same
    // common::errors::run as the standalone binaries: both paths
    // should produce identical --help output for the same applet.
    let direct = run(env!("CARGO_BIN_EXE_wc"), &["--help"]);
    let via_box = run(env!("CARGO_BIN_EXE_mitos-box"), &["wc", "--help"]);
    assert_eq!(stdout_of(&direct), stdout_of(&via_box));
}
