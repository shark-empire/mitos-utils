//! Integration tests for the text-processing utilities: echo,
//! printf, head, tail, grep, sort, uniq, wc, cut, tr, diff.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin).args(args).output().expect("spawn binary")
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

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn echo_prints_arguments_with_trailing_newline() {
    let out = run(env!("CARGO_BIN_EXE_echo"), &["hello", "world"]);
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "hello world\n");
}

#[test]
fn echo_dash_n_suppresses_newline() {
    let out = run(env!("CARGO_BIN_EXE_echo"), &["-n", "hi"]);
    assert_eq!(stdout_of(&out), "hi");
}

#[test]
fn echo_dash_e_expands_escapes() {
    let out = run(env!("CARGO_BIN_EXE_echo"), &["-e", "a\\tb"]);
    assert_eq!(stdout_of(&out), "a\tb\n");
}

#[test]
fn printf_substitutes_percent_s_and_percent_d() {
    let out = run(env!("CARGO_BIN_EXE_printf"), &["%s is %d\\n", "mitos", "1"]);
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "mitos is 1\n");
}

#[test]
fn head_default_returns_first_ten_lines() {
    let input: String = (1..=20).map(|n| format!("line{}\n", n)).collect();
    let out = run_with_stdin(env!("CARGO_BIN_EXE_head"), &[], &input);
    let output = stdout_of(&out);
     let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 10);
    assert_eq!(lines[0], "line1");
    assert_eq!(lines[9], "line10");
}

#[test]
fn head_dash_n_limits_output() {
    let input = "a\nb\nc\nd\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_head"), &["-n", "2"], input);
    assert_eq!(stdout_of(&out), "a\nb\n");
}

#[test]
fn tail_default_returns_last_ten_lines() {
    let input: String = (1..=20).map(|n| format!("line{}\n", n)).collect();
    let out = run_with_stdin(env!("CARGO_BIN_EXE_tail"), &[], &input);
    let output = stdout_of(&out);
     let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 10);
    assert_eq!(lines[0], "line11");
    assert_eq!(lines[9], "line20");
}

#[test]
fn tail_dash_n_limits_output() {
    let input = "a\nb\nc\nd\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_tail"), &["-n", "2"], input);
    assert_eq!(stdout_of(&out), "c\nd\n");
}

#[test]
fn grep_finds_matching_lines() {
    let input = "apple\nbanana\ngrape\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_grep"), &["an"], input);
    assert!(out.status.success());
    assert_eq!(stdout_of(&out), "banana\n");
}

#[test]
fn grep_no_match_exits_nonzero() {
    let input = "apple\nbanana\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_grep"), &["zzz"], input);
    assert!(!out.status.success());
    assert!(stdout_of(&out).is_empty());
}

#[test]
fn grep_dash_v_inverts_match() {
    let input = "apple\nbanana\ngrape\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_grep"), &["-v", "an"], input);
    assert_eq!(stdout_of(&out), "apple\ngrape\n");
}

#[test]
fn sort_orders_lines_alphabetically() {
    let input = "banana\napple\ncherry\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_sort"), &[], input);
    assert_eq!(stdout_of(&out), "apple\nbanana\ncherry\n");
}

#[test]
fn sort_dash_n_orders_numerically() {
    let input = "10\n2\n1\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_sort"), &["-n"], input);
    assert_eq!(stdout_of(&out), "1\n2\n10\n");
}

#[test]
fn sort_dash_r_reverses_order() {
    let input = "a\nb\nc\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_sort"), &["-r"], input);
    assert_eq!(stdout_of(&out), "c\nb\na\n");
}

#[test]
fn uniq_collapses_adjacent_duplicates() {
    let input = "a\na\nb\nb\nb\nc\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_uniq"), &[], input);
    assert_eq!(stdout_of(&out), "a\nb\nc\n");
}

#[test]
fn uniq_dash_c_prefixes_counts() {
    let input = "a\na\nb\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_uniq"), &["-c"], input);
    let text = stdout_of(&out);
    assert!(text.contains("2 a"));
    assert!(text.contains("1 b"));
}

#[test]
fn wc_counts_lines_words_bytes() {
    let input = "one two\nthree\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_wc"), &[], input);
    let text = stdout_of(&out);
    // 2 lines, 3 words, 14 bytes
    assert!(text.contains('2'));
    assert!(text.contains('3'));
    assert!(text.contains("14"));
}

#[test]
fn cut_extracts_requested_field() {
    let input = "a:b:c\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_cut"), &["-d", ":", "-f", "2"], input);
    assert_eq!(stdout_of(&out), "b\n");
}

#[test]
fn cut_supports_field_ranges() {
    let input = "a:b:c:d\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_cut"), &["-d", ":", "-f", "2-3"], input);
    assert_eq!(stdout_of(&out), "b:c\n");
}

#[test]
fn tr_translates_characters() {
    let out = run_with_stdin(env!("CARGO_BIN_EXE_tr"), &["ab", "xy"], "aabbcc");
    assert_eq!(stdout_of(&out), "xxyycc");
}

#[test]
fn tr_dash_d_deletes_characters() {
    let out = run_with_stdin(env!("CARGO_BIN_EXE_tr"), &["-d", "a"], "banana");
    assert_eq!(stdout_of(&out), "bnn");
}

#[test]
fn head_dash_c_limits_output_to_byte_count() {
    let input = "abcdefghij";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_head"), &["-c", "3"], input);
    assert_eq!(stdout_of(&out), "abc");
}

#[test]
fn tail_dash_c_returns_last_n_bytes_from_stdin() {
    let input = "abcdefghij";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_tail"), &["-c", "3"], input);
    assert_eq!(stdout_of(&out), "hij");
}

#[test]
fn tail_dash_c_returns_last_n_bytes_from_a_file() {
    let dir = std::env::temp_dir().join(format!("mitos-tail-c-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("data.txt");
    std::fs::write(&file, "abcdefghij").unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_tail"),
        &["-c", "4", file.to_str().unwrap()],
    );
    assert_eq!(stdout_of(&out), "ghij");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sort_dash_k_sorts_by_field() {
    // Sort by the second colon-delimited field.
    let input = "b:3\na:1\nc:2\n";
    let out = run_with_stdin(env!("CARGO_BIN_EXE_sort"), &["-t", ":", "-k", "2"], input);
    assert_eq!(stdout_of(&out), "a:1\nc:2\nb:3\n");
}

#[test]
fn sort_dash_k_dash_n_sorts_by_field_numerically() {
    let input = "x:10\ny:2\nz:1\n";
    let out = run_with_stdin(
        env!("CARGO_BIN_EXE_sort"),
        &["-t", ":", "-k", "2", "-n"],
        input,
    );
    assert_eq!(stdout_of(&out), "z:1\ny:2\nx:10\n");
}
#[test]
fn diff_identical_files_exits_zero_with_no_output() {
    let dir = std::env::temp_dir().join(format!("mitos-diff-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.txt");
    let b = dir.join("b.txt");
    std::fs::write(&a, "same\ncontent\n").unwrap();
    std::fs::write(&b, "same\ncontent\n").unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_diff"),
        &[a.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert!(out.status.success());
    assert!(stdout_of(&out).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn diff_differing_files_exits_nonzero_with_output() {
    let dir = std::env::temp_dir().join(format!("mitos-diff-test2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.txt");
    let b = dir.join("b.txt");
    std::fs::write(&a, "one\n").unwrap();
    std::fs::write(&b, "two\n").unwrap();

    let out = run(
        env!("CARGO_BIN_EXE_diff"),
        &[a.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert!(!out.status.success());
    assert!(!stdout_of(&out).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
