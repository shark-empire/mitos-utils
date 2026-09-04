//! `tr` -- translate or delete characters from stdin. Supports
//! `SET1 SET2` translation and `-d SET1` deletion. Sets are taken as
//! literal character lists (no `[a-z]`-style ranges -- see
//! docs/compatibility.md).

use crate::common::errors::{AppError, AppResult};
use std::io::{self, Read, Write};

pub const USAGE: &str = "tr SET1 SET2 | tr -d SET1 -- translate or delete characters on stdin";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut delete_mode = false;
    let mut rest: Vec<String> = Vec::new();
    for arg in args {
        if arg == "-d" {
            delete_mode = true;
        } else {
            rest.push(arg);
        }
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(AppError::from)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if delete_mode {
        let set1: Vec<char> = rest
            .first()
            .map(|s| s.chars().collect())
            .unwrap_or_default();
        write!(out, "{}", delete_chars(&input, &set1)).map_err(AppError::from)?;
        return Ok(());
    }

    if rest.len() != 2 {
        return Err(AppError::usage("usage: tr SET1 SET2  |  tr -d SET1"));
    }
    let set1: Vec<char> = rest[0].chars().collect();
    let set2: Vec<char> = rest[1].chars().collect();
    write!(out, "{}", translate(&input, &set1, &set2)).map_err(AppError::from)?;
    Ok(())
}

/// Remove every character in `set1` from `input`. Standalone (used
/// by both `run` and `fuzz/fuzz_targets/tr_translate.rs`).
pub fn delete_chars(input: &str, set1: &[char]) -> String {
    input.chars().filter(|c| !set1.contains(c)).collect()
}

/// Map each character of `input` that appears in `set1` to the
/// character at the same position in `set2` (or `set2`'s last
/// character if `set1` is longer than `set2`, matching GNU `tr`'s
/// own truncation behavior); characters not in `set1` pass through
/// unchanged. Standalone (used by both `run` and
/// `fuzz/fuzz_targets/tr_translate.rs` -- character-set indexing
/// logic like `set2.last()` is an easy place to get an off-by-one
/// wrong, worth fuzzing rather than only hand-testing).
pub fn translate(input: &str, set1: &[char], set2: &[char]) -> String {
    input
        .chars()
        .map(|c| match set1.iter().position(|&s| s == c) {
            Some(idx) => *set2.get(idx).or_else(|| set2.last()).unwrap_or(&c),
            None => c,
        })
        .collect()
}
