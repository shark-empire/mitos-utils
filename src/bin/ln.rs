//! `ln` -- create links. Hard links by default (POSIX behavior);
//! pass `-s`/`--symbolic` for a symbolic link instead, and
//! `-f`/`--force` to replace an existing `LINK_NAME`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use std::os::unix::fs::symlink;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("ln", real_main)
}

fn real_main() -> AppResult<()> {
    let mut symbolic = false;
    let mut force = false;
    let mut rest: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-s" | "--symbolic" => symbolic = true,
            "-f" | "--force" => force = true,
            _ => rest.push(arg),
        }
    }
    if rest.len() != 2 {
        return Err(AppError::usage("usage: ln [-s] [-f] TARGET LINK_NAME"));
    }
    let target = &rest[0];
    let link_name = &rest[1];

    if force && Path::new(link_name).exists() {
        std::fs::remove_file(link_name).map_err(|e| AppError::new(e.to_string()))?;
    }

    let result = if symbolic {
        symlink(target, link_name)
    } else {
        std::fs::hard_link(target, link_name)
    };

    result.map_err(|err| AppError::new(format!("failed to create link '{}': {}", link_name, err)))
}
