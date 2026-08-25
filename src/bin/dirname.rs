//! `dirname` -- strip the last path component, printing what's left.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::paths::dirname;

fn main() -> std::process::ExitCode {
    run("dirname", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err(AppError::usage("missing operand"));
    }
    for path in &args {
        println!("{}", dirname(path));
    }
    Ok(())
}
