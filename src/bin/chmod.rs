//! `chmod` -- change file mode bits. Accepts octal (`755`) or
//! symbolic (`u+x`, `go-w`) mode specs (see
//! `common::permissions::parse_mode`), and `-R`/`--recursive`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::error_path;
use mitos_utils::common::paths::walk;
use mitos_utils::common::permissions::parse_mode;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("chmod", real_main)
}

fn real_main() -> AppResult<()> {
    let mut recursive = false;
    let mut rest: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-R" | "--recursive" => recursive = true,
            _ => rest.push(arg),
        }
    }
    if rest.len() < 2 {
        return Err(AppError::usage("usage: chmod [-R] MODE FILE..."));
    }
    let spec = rest.remove(0);
    let files = rest;

    let mut had_error = false;
    for file in &files {
        let path = Path::new(file);
        let targets = if recursive {
            walk(path).map_err(|e| AppError::new(e.to_string()))?
        } else {
            vec![path.to_path_buf()]
        };
        for target in &targets {
            if let Err(err) = chmod_one(target, &spec) {
                error_path("chmod", target.display(), err);
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

fn chmod_one(path: &Path, spec: &str) -> std::io::Result<()> {
    let current = fs::metadata(path)?.permissions().mode() & 0o7777;
    let new_mode = parse_mode(spec, current)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    fs::set_permissions(path, fs::Permissions::from_mode(new_mode))
}
