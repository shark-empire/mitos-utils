//! Shared building blocks used by every utility in `src/bin/`, so
//! that ~50 small programs don't each reimplement the same error
//! handling, output formatting, permission math, path edge cases,
//! and user/group lookups. See docs/architecture.md for the full
//! rationale and docs/compatibility.md for what each module does and
//! doesn't cover.

pub mod errors;
pub mod output;
pub mod paths;
pub mod permissions;
pub mod users;
