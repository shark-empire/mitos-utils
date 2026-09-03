//! `env` -- print the current environment, one `KEY=value` per line.

use crate::common::errors::AppResult;

pub const USAGE: &str = "env -- print the environment";

pub fn run(_args: Vec<String>) -> AppResult<()> {
    let mut vars: Vec<(String, String)> = std::env::vars().collect();
    vars.sort();
    for (key, value) in vars {
        println!("{}={}", key, value);
    }
    Ok(())
}
