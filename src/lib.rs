//! mitos-utils: core system utilities for MITOS.
//!
//! This crate is a small shared library (`common`) plus one binary
//! per utility under `src/bin/`. Every utility follows the same
//! shape:
//!
//! ```ignore
//! use mitos_utils::common::errors::{run, AppResult};
//!
//! fn main() -> std::process::ExitCode {
//!     run("toolname", real_main)
//! }
//!
//! fn real_main() -> AppResult<()> {
//!     // ...
//!     Ok(())
//! }
//! ```
//!
//! `run` prints coreutils-style `"toolname: message"` errors to
//! stderr and maps them to the right process exit code, so
//! individual utilities never have to think about that plumbing.
//! See docs/architecture.md for why the crate is organized this way.

pub mod common;
