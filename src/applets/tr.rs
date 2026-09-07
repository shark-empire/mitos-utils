//! `tr` -- translate or delete characters from stdin. Supports
//! `SET1 SET2` translation and `-d SET1` deletion. Sets are taken as
//! literal character lists (no `[a-z]`-style ranges -- see
//! docs/compatibility.md).

use crate::common::errors::{AppError, AppResult};
use std::collections::{HashMap, HashSet};
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
        out.write_all(delete_chars(&input, &set1).as_bytes())
            .map_err(AppError::from)?;
        return Ok(());
    }

    if rest.len() != 2 {
        return Err(AppError::usage("usage: tr SET1 SET2  |  tr -d SET1"));
    }
    let set1: Vec<char> = rest[0].chars().collect();
    let set2: Vec<char> = rest[1].chars().collect();
    out.write_all(translate(&input, &set1, &set2).as_bytes())
        .map_err(AppError::from)?;
    Ok(())
}

/// Remove every character in `set1` from `input`. Standalone (used
/// by both `run` and `fuzz/fuzz_targets/tr_translate.rs`).
///
/// Builds a `HashSet` from `set1` once up front so membership is an
/// O(1) average-case lookup per input character instead of the O(k)
/// linear scan a plain `.contains()` on a `Vec` would do -- for a
/// large `set1` (or a large input) that's the difference between
/// O(n) and O(n * k) overall.
pub fn delete_chars(input: &str, set1: &[char]) -> String {
    let remove: HashSet<char> = set1.iter().copied().collect();
    input.chars().filter(|c| !remove.contains(c)).collect()
}

/// Map each character of `input` that appears in `set1` to the
/// character at the same position in `set2` (or `set2`'s last
/// character if `set1` is longer than `set2`, matching GNU `tr`'s
/// own truncation behavior); characters not in `set1` pass through
/// unchanged. Standalone (used by both `run` and
/// `fuzz/fuzz_targets/tr_translate.rs` -- character-set indexing
/// logic like `set2.last()` is an easy place to get an off-by-one
/// wrong, worth fuzzing rather than only hand-testing).
///
/// Builds a `HashMap<char, char>` from `set1`/`set2` once up front
/// (same reasoning as `delete_chars`): each input character is then
/// one O(1) average-case lookup instead of an O(k) scan of `set1`.
pub fn translate(input: &str, set1: &[char], set2: &[char]) -> String {
    let fallback = set2.last().copied();
    let mut table: HashMap<char, char> = HashMap::with_capacity(set1.len());
    for (idx, &s) in set1.iter().enumerate() {
        if table.contains_key(&s) {
            // First occurrence in `set1` wins -- matches the
            // original `.position()`-based lookup, which always
            // found the *first* matching index too.
            continue;
        }
        if let Some(mapped) = set2.get(idx).copied().or(fallback) {
            table.insert(s, mapped);
        }
    }
    input
        .chars()
        .map(|c| table.get(&c).copied().unwrap_or(c))
        .collect()
}
