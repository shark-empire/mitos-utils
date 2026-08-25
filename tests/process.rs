//! Integration tests for the process/system utilities. Values like
//! hostname, uid, or memory numbers are host-dependent, so these
//! tests check *shape* (exit code, non-empty output, expected
//! substrings) rather than exact values.

use std::process::{Command, Output};
use std::time::Instant;

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin).args(args).output().expect("spawn binary")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn true_exits_success() {
    let out = run(env!("CARGO_BIN_EXE_true"), &[]);
    assert!(out.status.success());
}

#[test]
fn sleep_pauses_for_roughly_the_requested_time() {
    let start = Instant::now();
    let out = run(env!("CARGO_BIN_EXE_sleep"), &["0.2"]);
    let elapsed = start.elapsed();
    assert!(out.status.success());
    assert!(elapsed.as_millis() >= 180, "slept for {:?}, expected >= 180ms", elapsed);
}

#[test]
fn sleep_rejects_garbage_input() {
    let out = run(env!("CARGO_BIN_EXE_sleep"), &["not-a-number"]);
    assert!(!out.status.success());
}

#[test]
fn uname_prints_something() {
    let out = run(env!("CARGO_BIN_EXE_uname"), &[]);
    assert!(out.status.success());
    assert!(!stdout_of(&out).trim().is_empty());
}

#[test]
fn hostname_prints_something() {
    let out = run(env!("CARGO_BIN_EXE_hostname"), &[]);
    assert!(out.status.success());
    assert!(!stdout_of(&out).trim().is_empty());
}

#[test]
fn env_lists_key_value_pairs() {
    let out = Command::new(env!("CARGO_BIN_EXE_env"))
        .env("MITOS_UTILS_TEST_VAR", "sentinel-value")
        .output()
        .expect("spawn env");
    assert!(out.status.success());
    assert!(stdout_of(&out).contains("MITOS_UTILS_TEST_VAR=sentinel-value"));
}

#[test]
fn printenv_prints_requested_variable() {
    let out = Command::new(env!("CARGO_BIN_EXE_printenv"))
        .env("MITOS_UTILS_TEST_VAR", "sentinel-value")
        .arg("MITOS_UTILS_TEST_VAR")
        .output()
        .expect("spawn printenv");
    assert!(out.status.success());
    assert_eq!(stdout_of(&out).trim(), "sentinel-value");
}

#[test]
fn printenv_missing_variable_exits_nonzero() {
    let out = run(env!("CARGO_BIN_EXE_printenv"), &["MITOS_UTILS_DEFINITELY_UNSET_VAR"]);
    assert!(!out.status.success());
}

#[test]
fn whoami_prints_a_username() {
    let out = run(env!("CARGO_BIN_EXE_whoami"), &[]);
    assert!(out.status.success());
    assert!(!stdout_of(&out).trim().is_empty());
}

#[test]
fn id_output_contains_uid_and_gid() {
    let out = run(env!("CARGO_BIN_EXE_id"), &[]);
    assert!(out.status.success());
    let text = stdout_of(&out);
    assert!(text.contains("uid="));
    assert!(text.contains("gid="));
}

#[test]
fn groups_prints_at_least_one_group() {
    let out = run(env!("CARGO_BIN_EXE_groups"), &[]);
    assert!(out.status.success());
    assert!(!stdout_of(&out).trim().is_empty());
}
