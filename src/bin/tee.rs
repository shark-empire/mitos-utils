//! `tee` -- copy stdin to stdout and to one or more files. Supports
//! `-a`/`--append`.

use mitos_utils::common::errors::{run, AppError, AppResult};
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};

fn main() -> std::process::ExitCode {
    run("tee", real_main)
}

fn real_main() -> AppResult<()> {
    let mut append = false;
    let mut files: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-a" | "--append" => append = true,
            _ => files.push(arg),
        }
    }

    let mut outputs: Vec<std::fs::File> = Vec::new();
    for path in &files {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(path)
            .map_err(|e| AppError::new(format!("{}: {}", path, e)))?;
        outputs.push(file);
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for line in stdin.lock().lines().flatten() {
        writeln!(lock, "{}", line).ok();
        for f in &mut outputs {
            writeln!(f, "{}", line).ok();
        }
    }
    Ok(())
}
