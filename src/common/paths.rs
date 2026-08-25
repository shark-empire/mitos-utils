//! Pure path-manipulation helpers shared by `basename`, `dirname`,
//! `pwd`, `realpath`, `readlink`, and the recursive utilities
//! (`cp -r`, `rm -r`, `chmod -R`, `chown -R`, `du`), so the exact
//! POSIX edge cases (trailing slashes, `.`/`..`, empty input) are
//! implemented once instead of ~10 times slightly differently.

use std::path::{Component, Path, PathBuf};

/// POSIX `basename`: last path component after stripping trailing
/// slashes. `basename("")` is `"."`; `basename("/")` is `"/"`.
pub fn basename(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rsplit_once('/') {
        Some((_, last)) => last.to_string(),
        None => trimmed.to_string(),
    }
}

/// POSIX `dirname`: everything before the last path component.
/// `dirname("")` is `"."`; `dirname("/")` and `dirname("/foo")` are
/// both `"/"`; `dirname("foo")` is `"."`.
pub fn dirname(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rsplit_once('/') {
        Some(("", _)) => "/".to_string(),
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

/// Resolve `path` (relative to `cwd` if not already absolute) to an
/// absolute, `.`/`..`-free form purely lexically, without requiring
/// any component to exist. This is the fallback `realpath` and
/// `readlink -f` use once `std::fs::canonicalize` fails because the
/// target (or a component of it) doesn't exist yet -- matching GNU
/// `realpath -m`.
pub fn normalize(path: &Path, cwd: &Path) -> PathBuf {
    let absolute = if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };
    let mut out = PathBuf::from("/");
    for comp in absolute.components() {
        match comp {
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// Recursively collect every path under `root` (root included),
/// parents before children -- suitable for `chmod -R`/`chown -R`
/// (order doesn't matter) and for `cp -r` (a directory must exist
/// before anything is copied into it). Symlinked directories are
/// listed but not traversed into, to avoid following cycles.
pub fn walk(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = vec![root.to_path_buf()];
    if root.is_dir() && !root.is_symlink() {
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                out.push(path.clone());
                if path.is_dir() && !path.is_symlink() {
                    stack.push(path);
                }
            }
        }
    }
    Ok(out)
}

/// Same set of paths as `walk`, ordered deepest-first so `rm -r` can
/// delete every child before the directory that contains it.
pub fn walk_post_order(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut all = walk(root)?;
    all.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    Ok(all)
}
