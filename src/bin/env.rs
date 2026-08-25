//! `env` -- print the current environment, one `KEY=value` per line.

use mitos_utils::common::errors::{run, AppResult};

fn main() -> std::process::ExitCode {
    run("env", real_main)
}

fn real_main() -> AppResult<()> {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort();
    for (key, value) in vars {
        println!("{}={}", key, value);
    }
    Ok(())
}
