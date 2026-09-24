//! `find` -- walk a directory tree and print paths matching simple
//! filters.
//!
//! Deliberately scoped to the filters covering the large majority of
//! real usage: `-name`, `-type`, `-maxdepth`, `-mindepth`. `-exec`,
//! `-size`, `-mtime`/`-newer`, and `[abc]`-style character classes in
//! `-name` patterns (only `*`/`?` are supported) are real gaps, not
//! silent ones -- see README.md's Known limitations. Traversal
//! itself reuses `common::paths::walk`, the same symlink-safe walk
//! `cp -r`/`chmod -R`/`chown -R` already rely on, rather than a new
//! one just for this.

use crate::common::errors::{AppError, AppResult};
use crate::common::output::stdout_writer;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const USAGE: &str =
    "find [PATH...] [-name PATTERN] [-type f|d|l] [-maxdepth N] [-mindepth N] -- walk a directory tree";

/// Minimal shell-glob match against a whole string: `*` matches any
/// run of characters (including none), `?` matches exactly one.
/// No character classes -- kept small on purpose, the same call this
/// crate already made for `grep`'s own regex engine.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => go(&p[1..], t) || (!t.is_empty() && go(p, &t[1..])),
            Some('?') => !t.is_empty() && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    go(&p, &t)
}

#[derive(Default)]
struct Filters {
    name: Option<String>,
    file_type: Option<char>, // 'f' | 'd' | 'l'
    max_depth: Option<usize>,
    min_depth: Option<usize>,
}

fn matches(path: &Path, depth: usize, filters: &Filters) -> AppResult<bool> {
    if let Some(max) = filters.max_depth {
        if depth > max {
            return Ok(false);
        }
    }
    if depth < filters.min_depth.unwrap_or(0) {
        return Ok(false);
    }
    if let Some(pattern) = &filters.name {
        let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !glob_match(pattern, base) {
            return Ok(false);
        }
    }
    if let Some(t) = filters.file_type {
        let meta = std::fs::symlink_metadata(path)
            .map_err(|e| AppError::new(format!("{}: {}", path.display(), e)))?;
        let ok = match t {
            'f' => meta.is_file(),
            'd' => meta.is_dir(),
            'l' => meta.file_type().is_symlink(),
            _ => true,
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

fn parse_depth(iter: &mut std::vec::IntoIter<String>, flag: &str) -> AppResult<usize> {
    iter.next()
        .ok_or_else(|| AppError::usage(format!("{flag} requires an argument")))?
        .parse::<usize>()
        .map_err(|_| AppError::usage(format!("{flag} requires a non-negative integer")))
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, forced) = crate::common::args::split_dashdash(args);
    let mut roots: Vec<String> = Vec::new();
    let mut filters = Filters::default();

    let mut iter = opts.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-name" => {
                filters.name = Some(
                    iter.next()
                        .ok_or_else(|| AppError::usage("-name requires an argument"))?,
                );
            }
            "-type" => {
                let t = iter
                    .next()
                    .ok_or_else(|| AppError::usage("-type requires an argument"))?;
                match t.as_str() {
                    "f" | "d" | "l" => filters.file_type = t.chars().next(),
                    other => return Err(AppError::usage(format!("unknown -type '{other}'"))),
                }
            }
            "-maxdepth" => filters.max_depth = Some(parse_depth(&mut iter, "-maxdepth")?),
            "-mindepth" => filters.min_depth = Some(parse_depth(&mut iter, "-mindepth")?),
            other if other.starts_with('-') => {
                return Err(AppError::usage(format!("unknown predicate '{other}'")));
            }
            other => roots.push(other.to_string()),
        }
    }
    roots.extend(forced);
    if roots.is_empty() {
        roots.push(".".to_string());
    }

    let mut out = stdout_writer();
    let mut had_error = false;
    for root in &roots {
        let root_path = PathBuf::from(root);
        let entries = match crate::common::paths::walk(&root_path) {
            Ok(entries) => entries,
            Err(e) => {
                crate::common::output::error_path("find", root, e);
                had_error = true;
                continue;
            }
        };
        let root_depth = root_path.components().count();
        for path in &entries {
            let depth = path.components().count().saturating_sub(root_depth);
            match matches(path, depth, &filters) {
                Ok(true) => {
                    let _ = writeln!(out, "{}", path.display());
                }
                Ok(false) => {}
                Err(e) => {
                    crate::common::output::error_path("find", path.display(), e);
                    had_error = true;
                }
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

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn glob_matches_star_and_question_mark() {
        assert!(glob_match("*.txt", "foo.txt"));
        assert!(!glob_match("*.txt", "foo.txtx"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(glob_match("*", ""));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exactly"));
    }
}
