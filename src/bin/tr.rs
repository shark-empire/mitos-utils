//! `tr` -- translate or delete characters from stdin. Supports
//! `SET1 SET2` translation and `-d SET1` deletion. Sets are taken as
//! literal character lists (no `[a-z]`-style ranges -- see
//! docs/compatibility.md).

use mitos_utils::common::errors::{run, AppError, AppResult};
use std::io::{self, Read, Write};

fn main() -> std::process::ExitCode {
    run("tr", real_main)
}

fn real_main() -> AppResult<()> {
    let mut delete_mode = false;
    let mut rest: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        if arg == "-d" {
            delete_mode = true;
        } else {
            rest.push(arg);
        }
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).map_err(AppError::from)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if delete_mode {
        let set1: Vec<char> = rest.first().map(|s| s.chars().collect()).unwrap_or_default();
        let filtered: String = input.chars().filter(|c| !set1.contains(c)).collect();
        write!(out, "{}", filtered).map_err(AppError::from)?;
        return Ok(());
    }

    if rest.len() != 2 {
        return Err(AppError::usage("usage: tr SET1 SET2  |  tr -d SET1"));
    }
    let set1: Vec<char> = rest[0].chars().collect();
    let set2: Vec<char> = rest[1].chars().collect();
    let translated: String = input
        .chars()
        .map(|c| match set1.iter().position(|&s| s == c) {
            Some(idx) => *set2.get(idx).or_else(|| set2.last()).unwrap_or(&c),
            None => c,
        })
        .collect();
    write!(out, "{}", translated).map_err(AppError::from)?;
    Ok(())
}
