//! `realpath` -- resolve a path to an absolute, symlink-free form.
//! Falls back to lexical normalization (`.`/`..` resolution without
//! requiring the path to exist) when `canonicalize` fails, matching
//! GNU `realpath -m`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::paths::normalize;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("realpath", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }
    let cwd = std::env::current_dir().map_err(|e| AppError::new(e.to_string()))?;

    // Like GNU `realpath -m`: prefer a real canonicalize (resolves
    // symlinks), but fall back to purely lexical normalization for
    // paths that don't exist yet rather than failing.
    for arg in &args {
        let path = Path::new(arg);
        match path.canonicalize() {
            Ok(resolved) => println!("{}", resolved.display()),
            Err(_) => println!("{}", normalize(path, &cwd).display()),
        }
    }
    Ok(())
}
