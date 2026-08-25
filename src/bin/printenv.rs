//! `printenv` -- print the value of named environment variables (or
//! every variable if none are named).

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("printenv", real_main)
}

fn real_main() -> AppResult<()> {
    let names: Vec<String> = std::env::args().skip(1).collect();
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
