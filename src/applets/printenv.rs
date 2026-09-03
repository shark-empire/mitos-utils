//! `printenv` -- print the value of named environment variables (or
//! every variable if none are named).

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str = "printenv [NAME...] -- print named environment variables (or all)";

pub fn run(names: Vec<String>) -> AppResult<()> {
    if names.is_empty() {
        let mut vars: Vec<(String, String)> = std::env::vars().collect();
        vars.sort();
        for (key, value) in vars {
            println!("{}={}", key, value);
        }
        return Ok(());
    }

    let mut all_found = true;
    for name in &names {
        match std::env::var(name) {
            Ok(value) => println!("{}", value),
            Err(_) => all_found = false,
        }
    }
    if all_found {
        Ok(())
    } else {
        Err(AppError::silent(1))
    }
}
