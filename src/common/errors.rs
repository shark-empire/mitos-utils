//! Shared error handling and exit-code conventions.
//!
//! Every utility in this crate follows the same failure contract:
//! print `"<prog>: <message>"` to stderr and exit with one of the
//! codes below, mirroring POSIX/GNU coreutils conventions so shell
//! scripts and `tests/compatibility.rs` can rely on them.

use std::fmt;
use std::process::ExitCode;

/// Utility completed successfully.
pub const EXIT_SUCCESS: u8 = 0;
/// Utility failed for a general reason (bad input, I/O error, a
/// child operation failed, etc).
pub const EXIT_FAILURE: u8 = 1;
/// Utility was invoked with invalid arguments or unknown options.
pub const EXIT_USAGE: u8 = 2;

/// A single error type shared by every binary in this crate.
///
/// With ~50 small utilities, modeling every possible failure as a
/// distinct enum variant would be a lot of surface for very little
/// benefit. Instead this captures "what went wrong" as a message
/// plus the exit code that should be used for it.
#[derive(Debug)]
pub struct AppError {
    pub message: String,
    pub code: u8,
}

impl AppError {
    /// A general failure (exit code 1) with a message that `run`
    /// will print as `"<prog>: <message>"`.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), code: EXIT_FAILURE }
    }

    /// A bad-arguments failure (exit code 2).
    pub fn usage(message: impl Into<String>) -> Self {
        Self { message: message.into(), code: EXIT_USAGE }
    }

    /// A failure with an explicit exit code.
    pub fn with_code(message: impl Into<String>, code: u8) -> Self {
        Self { message: message.into(), code }
    }

    /// A failure that has already been reported (e.g. per-file
    /// errors printed in a loop over several arguments) -- `run`
    /// will exit with `code` but print nothing further.
    pub fn silent(code: u8) -> Self {
        Self { message: String::new(), code }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::new(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Standard `main()` wrapper: runs `body`, and on `Err` prints
/// `"<prog>: <message>"` (unless the message is empty) and returns
/// the matching process exit code.
///
/// ```ignore
/// fn main() -> std::process::ExitCode {
///     mitos_utils::common::errors::run("cat", real_main)
/// }
/// ```
pub fn run(prog: &str, body: impl FnOnce() -> AppResult<()>) -> ExitCode {
    crate::common::output::reset_sigpipe();
    match body() {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err(err) => {
            if !err.message.is_empty() {
                crate::common::output::error(prog, &err.message);
            }
            ExitCode::from(err.code)
        }
    }
}
