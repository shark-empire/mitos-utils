//! `basename` -- strip directory and (optionally) a trailing suffix
//! from a path.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::paths::basename;

fn main() -> std::process::ExitCode {
    run("basename", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }

    let mut name = basename(&args[0]);
    if let Some(suffix) = args.get(1) {
        if !suffix.is_empty() && name != *suffix {
            if let Some(stripped) = name.strip_suffix(suffix.as_str()) {
                name = stripped.to_string();
            }
        }
    }
    println!("{}", name);
    Ok(())
}
