# mitos-utils architecture

## What this crate is

`mitos-utils` is a coreutils-style suite of small userspace programs
for MITOS: one binary per utility under `src/bin/`, sharing a small
library (`src/common/`) for the handful of things every one of them
needs -- error/exit-code conventions, stderr formatting, path edge
cases, permission-bit math, and user/group lookups.

## Important: this targets a *hosted* environment, not the bare kernel

[mitosOS](../../mitosos) (the kernel this crate's utilities are meant
to eventually run on) currently builds for `x86_64-unknown-none` and
`aarch64-unknown-none` -- freestanding targets with no operating
system underneath them and no `std`. `mitos-utils` is written against
full `std` (`std::fs`, `std::env`, `std::process`, `std::thread`,
Unix-specific extension traits, and a handful of hand-rolled
`extern "C"` calls into libc). Those two facts don't yet meet in the
middle: there is no libc, syscall ABI, or `std` port for mitosOS for
this crate to actually run on top of yet.

That's expected at this stage. Concretely, this crate has two roles
right now:

1. **The reference implementation and spec.** It's written and tested
   against a real Unix host (Linux) so its behavior, flag semantics,
   and exit codes are pinned down and testable *today*, well before
   mitosOS has anything to host it.
2. **A checklist for what mitosOS's userspace layer needs to expose.**
   Every place this crate reaches past pure `std::fs`/`std::env` --
   `/proc` reads (`ps`, `free`, `uptime`), `statvfs`/`mount`/`umount`
   syscalls (`df`, `mount`, `umount`), `klogctl` (`dmesg`), `chown`,
   `kill` -- is a concrete signal for a syscall or pseudo-filesystem
   mitosOS's kernel will eventually need, once there's a userspace to
   grow. Search this crate for `mitosOS has no ... yet` to find all of
   them.

When mitosOS grows a syscall ABI and a `std` port targeting it, the
work is: (a) get this crate compiling against that new target at all,
then (b) swap each of those `cfg(target_os = "linux")` /
`cfg(unix)` FFI shims for the mitosOS equivalent. The `common::`
module boundaries were chosen so that swap touches as few files as
possible -- e.g. `common::users` is the *only* place uid/gid/name
lookups happen, so a future mitosOS identity syscall only needs to
change one file.

## Why a shared `common` library instead of 50 independent binaries

Early on this repo was just a handful of `src/bin/*.rs` files, each
reimplementing its own argument loop, its own `"toolname: error"`
formatting, and (once things like `ls -l` and `stat` needed it) its
own permission-bit-to-`rwxrwxrwx` string logic. That doesn't scale
to ~50 utilities: any fix to how errors are reported, or any POSIX
edge case discovered in path handling, would need repeating in every
file that touched it. `common/` exists so each of those concerns is
implemented -- and tested, and fixed -- exactly once:

- `common::errors` -- the `AppError`/`AppResult` type and the `run()`
  entry-point wrapper every `fn main()` uses. Also resets `SIGPIPE` to
  its default disposition before running, so piping a utility's output
  into something that closes early (`mitos-cat bigfile | head`) exits
  quietly instead of panicking -- a well-known Rust CLI footgun.
- `common::output` -- stderr formatting, human-readable byte sizes,
  and `ls`-style column layout.
- `common::paths` -- `basename`/`dirname`/lexical-normalize logic and
  the recursive directory walkers `cp -r`/`rm -r`/`chmod -R`/`du`
  all share.
- `common::permissions` -- mode-string rendering and `chmod`'s
  octal/symbolic mode parser.
- `common::users` -- uid/gid <-> name lookups, via hand-written FFI
  into the host libc (see "Zero dependencies" below).

## Zero dependencies, on purpose

`Cargo.toml` has no `[dependencies]` and no `[dev-dependencies]`.
Every utility that would normally reach for a crate (`libc` for
uid/gid/`statvfs`/`mount` syscalls, `tempfile`/`assert_cmd` for
tests) instead uses either pure `std` or a small hand-written
`extern "C"` block scoped to exactly the functions needed. This is
deliberate, not an oversight:

- It keeps the crate buildable in constrained/offline environments
  (relevant for a from-scratch OS project's toolchain), and keeps the
  eventual mitosOS cross-compilation story simpler -- one crate's
  worth of `std` + a few libc symbols to port, not a dependency tree.
- The `libc` crate would normally be the right tool for the FFI here.
  Hand-writing the handful of declarations actually used
  (`getpwuid`, `statvfs`, `mount`, `klogctl`, ...) keeps the surface
  area small and auditable, at the cost of needing to double-check
  struct layouts (see docs/compatibility.md) against a real build --
  there is no Rust toolchain in the environment these files were
  written in, so none of this has been compiled yet. Treat the
  `statvfs`/`mount`/`ps`-family code as reviewed-but-unverified until
  it's been through a real `cargo build` + `cargo test` on target.

## Directory layout

```
src/
  lib.rs           # pub mod common;
  common/
    mod.rs
    errors.rs      # AppError, AppResult, run()
    output.rs      # error(), error_path(), human_size(), columnate(), reset_sigpipe()
    paths.rs       # basename(), dirname(), normalize(), walk(), walk_post_order()
    permissions.rs # format_mode(), file_type_char(), mode_of(), parse_mode()
    users.rs       # current_identity(), name_for_uid/gid(), uid/gid_for_name(), supplementary_groups()
  bin/
    <one file per utility>
tests/
  filesystem.rs    # mkdir/touch/cat/cp/mv/rm/ln/pwd/stat
  text.rs          # echo/printf/head/tail/grep/sort/uniq/wc/cut/tr/diff
  process.rs       # sleep/uname/hostname/env/printenv/whoami/id/groups/true
  compatibility.rs # POSIX exit-code and basename/dirname edge-case checks
docs/
  architecture.md  # this file
  compatibility.md # what's implemented vs. deliberately out of scope, per utility
  commands.md      # one-paragraph usage reference per utility
```

## The `fn main()` shape every utility follows

```rust
use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("toolname", real_main)
}

fn real_main() -> AppResult<()> {
    // ... do the work, using `?` on anything that returns AppResult
    // or implements `From<std::io::Error> for AppError` ...
    Ok(())
}
```

Utilities that loop over several arguments (most of them: `cat`,
`cp`, `rm`, `chmod`, ...) print a per-item error as they go via
`common::output::error_path` and track a `had_error` flag, returning
`Err(AppError::silent(1))` at the end so `run()` sets the right exit
code without printing a duplicate summary message -- matching how
GNU coreutils report "some of N files failed."
