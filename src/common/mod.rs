//! Shared building blocks used by every utility in `src/bin/`, so
//! that ~50 small programs don't each reimplement the same error
//! handling, output formatting, permission math, path edge cases,
//! and user/group lookups. See docs/architecture.md for the full
//! rationale and docs/compatibility.md for what each module does and
//! doesn't cover.

pub mod args;
pub mod errors;
pub mod output;
pub mod paths;
pub mod permissions;
// TOCTOU-hardened recursive directory ops (see safewalk.rs's own
// module docs for why this is Linux-only): the FFI it wraps
// (openat/unlinkat/fchmodat/fchownat) is only declared under
// cfg(target_os = "linux") internally, so the module itself is gated
// here rather than leaving half-broken items for other targets to
// trip over.
#[cfg(target_os = "linux")]
pub mod safewalk;
pub mod users;
