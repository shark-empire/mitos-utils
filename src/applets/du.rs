//! `du` -- estimate file/directory space usage by summing file
//! sizes recursively (no `statvfs`/block-count math needed here --
//! that's `df`'s job). Supports `-h`/`--human-readable` and
//! `-s`/`--summarize` (print only the total for each argument
//! instead of every subdirectory).

use crate::common::errors::{AppError, AppResult};
use crate::common::output::{error_path, human_size, stdout_writer};
use crate::common::paths::walk;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "du [-h] [-s] [PATH...] -- estimate directory space usage";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut human = false;
    let mut summarize = false;
    let mut targets: Vec<String> = Vec::new();

    for arg in args {
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
    let mut out = stdout_writer();
    for target in &targets {
        let path = Path::new(target);
        match du_one(path, summarize) {
            Ok(sizes) => {
                for (p, size) in sizes {
                    let rendered = if human {
                        human_size(size)
                    } else {
                        size.to_string()
                    };
                    let _ = writeln!(out, "{:<10} {}", rendered, p.display());
                }
            }
            Err(err) => {
                error_path("du", target, err);
                had_error = true;
            }
        }
    }
    let _ = out.flush();

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
    let mut dir_totals: BTreeMap<PathBuf, u64> = BTreeMap::new();
    for entry in &entries {
        if entry.is_dir() {
            dir_totals.entry(entry.clone()).or_insert(0);
        }
    }
    dir_totals.entry(root.to_path_buf()).or_insert(0);

    // Add each file's size to every ancestor directory's running
    // total by walking up its own parent chain, instead of the
    // previous approach of comparing every file against every
    // directory in the tree (`for file { for dir { .. } }`, an
    // O(files * directories) scan). Walking ancestors is
    // O(files * depth) -- for a wide tree with many top-level
    // directories but only a handful of nesting levels, that's a
    // very different growth curve.
    for entry in &entries {
        let meta = std::fs::symlink_metadata(entry)?;
        if !meta.is_file() {
            continue;
        }
        let size = meta.len();
        let mut dir = entry.parent();
        while let Some(d) = dir {
            if let Some(total) = dir_totals.get_mut(d) {
                *total += size;
            }
            if d == root {
                break;
            }
            dir = d.parent();
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
