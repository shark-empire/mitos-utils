//! `du` -- estimate file/directory space usage by summing file
//! sizes recursively (no `statvfs`/block-count math needed here --
//! that's `df`'s job). Supports `-h`/`--human-readable` and
//! `-s`/`--summarize` (print only the total for each argument
//! instead of every subdirectory).

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::{error_path, human_size};
use mitos_utils::common::paths::walk;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() -> std::process::ExitCode {
    run("du", real_main)
}

fn real_main() -> AppResult<()> {
    let mut human = false;
    let mut summarize = false;
    let mut targets: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--human-readable" => human = true,
            "-s" | "--summarize" => summarize = true,
            _ => targets.push(arg),
        }
    }
    if targets.is_empty() {
        targets.push(".".to_string());
    }

    let mut had_error = false;
    for target in &targets {
        let path = Path::new(target);
        match du_one(path, summarize) {
            Ok(sizes) => {
                for (p, size) in sizes {
                    let rendered = if human { human_size(size) } else { size.to_string() };
                    println!("{:<10} {}", rendered, p.display());
                }
            }
            Err(err) => {
                error_path("du", target, err);
                had_error = true;
            }
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

/// Returns `(path, total_bytes)` pairs: just the root if
/// `summarize`, otherwise every directory under it (deepest first),
/// each annotated with the size of everything beneath it.
fn du_one(root: &Path, summarize: bool) -> std::io::Result<Vec<(PathBuf, u64)>> {
    let entries = walk(root)?;
    let mut file_sizes: BTreeMap<PathBuf, u64> = BTreeMap::new();
    for entry in &entries {
        let meta = std::fs::symlink_metadata(entry)?;
        if meta.is_file() {
            file_sizes.insert(entry.clone(), meta.len());
        }
    }

    let mut dir_totals: BTreeMap<PathBuf, u64> = BTreeMap::new();
    for entry in &entries {
        if entry.is_dir() {
            dir_totals.insert(entry.clone(), 0);
        }
    }
    dir_totals.entry(root.to_path_buf()).or_insert(0);

    for (file, size) in &file_sizes {
        for (dir, total) in dir_totals.iter_mut() {
            if file.starts_with(dir) {
                *total += size;
            }
        }
    }

    if summarize {
        let total = dir_totals.get(root).copied().unwrap_or(0);
        return Ok(vec![(root.to_path_buf(), total)]);
    }

    let mut out: Vec<(PathBuf, u64)> = dir_totals.into_iter().collect();
    out.sort_by_key(|(p, _)| std::cmp::Reverse(p.components().count()));
    Ok(out)
}
