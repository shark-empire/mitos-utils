//! Compatibility checks against POSIX/GNU-documented behavior that's
//! deterministic and platform-independent -- exit-code conventions
//! and the well-known `basename`/`dirname` edge-case table (POSIX
//! XCU `basename`/`dirname`: trailing slashes are stripped before
//! taking the last component; `/` and `.` and `..` and the empty
//! string are all special-cased). See docs/compatibility.md for the
//! parts of each spec this crate deliberately doesn't implement.

use std::process::{Command, Output};

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin).args(args).output().expect("spawn binary")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn basename_of(input: &str) -> String {
    stdout_of(&run(env!("CARGO_BIN_EXE_basename"), &[input]))
        .trim()
        .to_string()
}

fn dirname_of(input: &str) -> String {
    stdout_of(&run(env!("CARGO_BIN_EXE_dirname"), &[input]))
        .trim()
        .to_string()
}

#[test]
fn basename_matches_posix_table() {
    assert_eq!(basename_of("/usr/lib"), "lib");
    assert_eq!(basename_of("/usr/"), "usr");
    assert_eq!(basename_of("usr"), "usr");
    assert_eq!(basename_of("/"), "/");
    assert_eq!(basename_of("."), ".");
    assert_eq!(basename_of(".."), "..");
}

#[test]
fn dirname_matches_posix_table() {
    assert_eq!(dirname_of("/usr/lib"), "/usr");
    assert_eq!(dirname_of("/usr/"), "/");
    assert_eq!(dirname_of("usr"), ".");
    assert_eq!(dirname_of("/"), "/");
    assert_eq!(dirname_of("."), ".");
    assert_eq!(dirname_of(".."), ".");
}

#[test]
fn basename_strips_matching_suffix() {
    let out = run(
        env!("CARGO_BIN_EXE_basename"),
        &["/usr/include/stdio.h", ".h"],
    );
    assert_eq!(stdout_of(&out).trim(), "stdio");
}

#[test]
fn basename_does_not_strip_suffix_if_result_would_be_empty() {
    // POSIX: if the suffix equals the whole basename, it is not
    // stripped (basename would otherwise become the empty string).
    let out = run(env!("CARGO_BIN_EXE_basename"), &["/usr/.h", ".h"]);
    assert_eq!(stdout_of(&out).trim(), ".h");
}

#[test]
fn true_exits_zero() {
    let status = run(env!("CARGO_BIN_EXE_true"), &[]).status;
    assert_eq!(status.code(), Some(0));
}

#[test]
fn missing_file_operations_exit_nonzero() {
    let missing = "/definitely/does/not/exist/mitos-utils-test";
    assert_ne!(
        run(env!("CARGO_BIN_EXE_cat"), &[missing]).status.code(),
        Some(0)
    );
    assert_ne!(
        run(env!("CARGO_BIN_EXE_stat"), &[missing]).status.code(),
        Some(0)
    );
    assert_ne!(
        run(env!("CARGO_BIN_EXE_rm"), &[missing]).status.code(),
        Some(0)
    );
}

#[test]
fn grep_exit_codes_follow_gnu_convention() {
    // 0 = match found, 1 = no match, 2 = usage/file error.
    let dir = std::env::temp_dir().join(format!("mitos-compat-grep-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("haystack.txt");
    std::fs::write(&file, "needle\n").unwrap();

    let found = run(
        env!("CARGO_BIN_EXE_grep"),
        &["needle", file.to_str().unwrap()],
    );
    assert_eq!(found.status.code(), Some(0));

    let not_found = run(
        env!("CARGO_BIN_EXE_grep"),
        &["missing", file.to_str().unwrap()],
    );
    assert_eq!(not_found.status.code(), Some(1));

    let bad_file = run(env!("CARGO_BIN_EXE_grep"), &["needle", "/no/such/file"]);
    assert_eq!(bad_file.status.code(), Some(2));

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn chmod_octal_mode_is_applied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("mitos-compat-chmod-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("perm.txt");
    std::fs::write(&file, "x").unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_chmod"),
        &["600", file.to_str().unwrap()],
    );
    assert!(out.status.success());
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn chmod_symbolic_mode_is_applied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("mitos-compat-chmod-sym-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("perm.txt");
    std::fs::write(&file, "x").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_chmod"),
        &["u+x", file.to_str().unwrap()],
    );
    assert!(out.status.success());
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o744);

    let _ = std::fs::remove_dir_all(&dir);
}
