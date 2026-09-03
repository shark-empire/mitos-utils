//! `realpath` -- resolve a path to an absolute, symlink-free form.
//! Falls back to lexical normalization (`.`/`..` resolution without
//! requiring the path to exist) when `canonicalize` fails, matching
//! GNU `realpath -m`.

use crate::common::errors::{AppError, AppResult};
use crate::common::paths::normalize;
use std::path::Path;

pub const USAGE: &str = "realpath PATH... -- resolve to an absolute, symlink-free path";

pub fn run(args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }
    let cwd = std::env::current_dir().map_err(|e| AppError::new(e.to_string()))?;

    for arg in &args {
        let path = Path::new(arg);
        match path.canonicalize() {
            Ok(resolved) => println!("{}", resolved.display()),
            Err(_) => println!("{}", normalize(path, &cwd).display()),
        }
    }
    Ok(())
}
