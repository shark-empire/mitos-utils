//! `nice` -- run a command at an adjusted scheduling priority.

use crate::common::errors::{AppError, AppResult};

pub const USAGE: &str =
    "nice [-n ADJUSTMENT] COMMAND [ARGS...] -- run COMMAND at an adjusted priority (default +10)";

#[cfg(unix)]
mod ffi {
    use std::os::raw::c_int;
    extern "C" {
        pub fn setpriority(which: c_int, who: u32, prio: c_int) -> c_int;
    }
}

#[cfg(unix)]
const PRIO_PROCESS: std::os::raw::c_int = 0;

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut adjustment: i32 = 10;
    let mut start = 0;

    if args.first().map(String::as_str) == Some("-n") {
        let value = args
            .get(1)
            .ok_or_else(|| AppError::usage("-n requires an argument"))?;
        adjustment = value
            .parse()
            .map_err(|_| AppError::usage("-n requires an integer"))?;
        start = 2;
    }

    let (program, cmd_args) = args
        .get(start..)
        .and_then(|rest| rest.split_first())
        .ok_or_else(|| AppError::usage("missing command"))?;

    set_priority(adjustment)?;
    exec_command(program, cmd_args)
}

#[cfg(unix)]
fn set_priority(adjustment: i32) -> AppResult<()> {
    // setpriority's "who=0" means "the calling process itself".
    if unsafe { ffi::setpriority(PRIO_PROCESS, 0, adjustment) } != 0 {
        return Err(AppError::new(format!(
            "cannot set priority to {adjustment}: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_priority(_adjustment: i32) -> AppResult<()> {
    Err(AppError::new(
        "priority adjustment not available on this target",
    ))
}

#[cfg(unix)]
fn exec_command(program: &str, args: &[String]) -> AppResult<()> {
    use std::os::unix::process::CommandExt;
    // exec() only returns on failure -- a successful run never comes
    // back here.
    let err = std::process::Command::new(program).args(args).exec();
    Err(AppError::new(format!("cannot run '{program}': {err}")))
}

#[cfg(not(unix))]
fn exec_command(_program: &str, _args: &[String]) -> AppResult<()> {
    Err(AppError::new(
        "running a replacement process not available on this target",
    ))
}
