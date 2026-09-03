# mitos-utils architecture

## What this crate is

`mitos-utils` is a coreutils-style suite of ~50 small userspace
utilities for MITOS. Each one's logic lives in
`src/applets/<name>.rs` as a plain callable function; `src/bin/`
holds both a thin per-utility wrapper binary for each one *and*
`mitos-box`, a single multiplexed binary that can act as any of them
(busybox/toybox-style -- see "One binary, many names" below). All of
it shares a small library (`src/common/`) for the handful of things
every applet needs -- error/exit-code conventions, stderr formatting,
path edge cases, permission-bit math, user/group lookups, and
TOCTOU-safe recursive directory operations.

See docs/integration.md for the full callable API surface if you're
integrating mitos-utils into another MITOS crate rather than just
building it.

## Important: this targets a *hosted* environment, not the bare kernel

[mitosOS](../../mitosos) (the kernel this crate's utilities are meant
to eventually run on) currently builds for `x86_64-unknown-none` and
`aarch64-unknown-none` -- freestanding targets with no operating
system underneath them and no `std`. `mitos-utils` is written against
full `std` (`std::fs`, `std::env`, `std::process`, `std::thread`,
Unix-specific extension traits, and a handful of hand-rolled
`extern "C"` calls into libc). Those two facts don't yet meet in the
middle: there is no libc, syscall ABI, or `std` port for mitosOS for
this crate to actually run on top of yet -- and no `fork`/`exec`
either, which is why docs/integration.md leads with linking this
crate in as a library rather than spawning its binaries.

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
   them, or see the consolidated list in docs/integration.md.

When mitosOS grows a syscall ABI and a `std` port targeting it, the
work is: (a) get this crate compiling against that new target at all,
then (b) swap each of those `cfg(target_os = "linux")` /
`cfg(unix)` FFI shims for the mitosOS equivalent. The `common::`
module boundaries were chosen so that swap touches as few files as
possible -- e.g. `common::users` is the *only* place uid/gid/name
lookups happen, so a future mitosOS identity syscall only needs to
change one file.

## Why `applets/` is separate from `bin/`

Originally each utility's logic lived directly in its
`src/bin/<name>.rs`, reading `std::env::args()` itself. That works
fine for 50 standalone binaries, but it means the *logic* isn't
callable except by spawning a process -- a real cost on a from-scratch
OS that doesn't have `fork`/`exec` yet (see docs/integration.md), and
it also means the only way to get one binary that can act as several
tools (see "One binary, many names" below) would be to duplicate
every `match` statement into a giant dispatcher.

So each applet's real work is a plain function,
`pub fn run(args: Vec<String>) -> AppResult<()>`, living in
`src/applets/<name>.rs` and taking its arguments as a parameter
instead of reading the process environment directly. Every
`src/bin/<name>.rs` is now five lines:

```rust
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("cat", mitos_utils::applets::cat::USAGE, args, mitos_utils::applets::cat::run)
}
```

`common::errors::run` is the one place `--help`/`--version` are
handled and errors get reported -- see "Centralized `--help` and
`--version`" below. Existing integration tests (`env!("CARGO_BIN_EXE_*")`)
didn't need to change at all: the standalone binaries still exist and
still behave the same, they just call into `applets::` now instead of
containing the logic directly.

## One binary, many names: `mitos-box`

`src/bin/mitos-box.rs` is a single binary that can act as *any*
applet, by looking at how it was invoked:

- **As a symlink/hardlink** named after an applet (`ln -s mitos-box
  cat`, matching busybox/toybox's own convention) -- it reads
  `argv[0]`'s basename and dispatches straight to that applet.
- **Invoked directly** with the applet name as the first argument
  (`mitos-box ls -la`) -- useful without setting up symlinks.

The point: a real mitosOS install can ship one `mitos-box` binary
plus ~50 symlinks pointing at it, instead of ~50 separate copies of
the same statically-linked `std` runtime -- the same footprint
argument busybox and toybox make for embedded Linux applies just as
much to a from-scratch OS's initial disk image. `applets::APPLETS`
(`src/applets/mod.rs`) is the `(name, usage, run)` table `mitos-box`
dispatches through; see docs/integration.md for how another crate
could use that same table instead of reimplementing dispatch.

## Centralized `--help` and `--version`

Previously *no* utility supported either flag. `common::errors::run`
now checks for bare `--help`/`--version` before calling into the
applet at all, so this is handled once instead of ~50 times. It
deliberately checks only the long form `--help`, never `-h`: several
applets already use `-h` for their own purposes (`ls -h`, `du -h` =
human-readable sizes), and GNU coreutils resolves the identical
conflict the same way.

## POSIX `--` (end of options)

Also previously supported by *no* utility: without it, nothing could
operate on a file literally named e.g. `-oddfile` (every arg loop
would try to interpret it as a flag). `common::args::split_dashdash`
splits an argument list on the first bare `--` token; applets that
take file/path positional arguments call it once at the top of their
own parsing loop and append the post-`--` arguments unfiltered to
whatever they collect. See `tests/cli_infra.rs` for the resulting
behavior.

## TOCTOU-safe recursive operations (`common::safewalk`)

`rm -r`, `chmod -R`, and `chown -R`/`chgrp -R` recurse through a
directory tree and act on what they find -- the classic risk being
that something can change *between* checking an entry and acting on
it (e.g. swapping a directory for a symlink mid-traversal, redirecting
a delete or a permission change somewhere never intended). On Linux,
these three now route through `common::safewalk`, which closes that
window via a check-open-verify pattern built entirely on
architecture-independent primitives -- deliberately *not*
`O_DIRECTORY`/`O_NOFOLLOW`, whose numeric values differ between x86_64
and arm64 on Linux (confirmed via a real 2024 QEMU bug report, not
just written from memory -- see `safewalk.rs`'s own doc comment for
the full design writeup, including why this matters given mitosOS
targets both architectures). Other targets fall back to the original
plain path-based walk (`common::paths::walk`/`walk_post_order`).

## Why a shared `common` library instead of 50 independent binaries

Early on this repo was just a handful of `src/bin/*.rs` files, each
reimplementing its own argument loop, its own `"toolname: error"`
formatting, and (once things like `ls -l` and `stat` needed it) its
own permission-bit-to-`rwxrwxrwx` string logic. That doesn't scale
to ~50 utilities: any fix to how errors are reported, or any POSIX
edge case discovered in path handling, would need repeating in every
file that touched it. `common/` exists so each of those concerns is
implemented -- and tested, and fixed -- exactly once. See
docs/integration.md for the full function-by-function reference;
briefly, by module:

- `common::errors` -- the `AppError`/`AppResult` type and the `run()`
  entry-point every `fn main()` (and `mitos-box`) calls through.
  Handles `--help`/`--version` and resets `SIGPIPE` to its default
  disposition before running, so piping a utility's output into
  something that closes early (`mitos-cat bigfile | head`) exits
  quietly instead of panicking -- a well-known Rust CLI footgun.
- `common::output` -- stderr formatting, the `-i` confirmation
  prompt, human-readable byte sizes, and `ls`-style column layout.
- `common::paths` -- `basename`/`dirname`/lexical-normalize logic and
  the recursive directory walkers `cp -r`/`du`/non-Linux
  `rm -r`/`chmod -R`/`chown -R` fall back to.
- `common::permissions` -- mode-string rendering and `chmod`'s
  octal/symbolic mode parser.
- `common::users` -- uid/gid <-> name lookups, via hand-written FFI
  into the host libc (see "Zero dependencies" below).
- `common::args` -- the `--` end-of-options splitter.
- `common::safewalk` -- TOCTOU-safe recursive directory operations
  (Linux only; see above).

## Zero dependencies, on purpose (with one deliberate exception)

The main crate's `Cargo.toml` has no `[dependencies]` and no
`[dev-dependencies]`. Every utility that would normally reach for a
crate (`libc` for uid/gid/`statvfs`/`mount` syscalls,
`tempfile`/`assert_cmd` for tests) instead uses either pure `std` or
a small hand-written `extern "C"` block scoped to exactly the
functions needed. This is deliberate, not an oversight:

- It keeps the crate buildable in constrained/offline environments
  (relevant for a from-scratch OS project's toolchain), and keeps the
  eventual mitosOS cross-compilation story simpler -- one crate's
  worth of `std` + a few libc symbols to port, not a dependency tree.
- The `libc` crate would normally be the right tool for the FFI here.
  Hand-writing the handful of declarations actually used
  (`getpwuid`, `statvfs`, `mount`, `klogctl`, `openat`, ...) keeps the
  surface area small and auditable, at the cost of needing to
  double-check struct layouts and flag values (see
  docs/compatibility.md) against real headers -- there is no Rust
  toolchain in the environment these files were written in, so none
  of this has been compiled yet. `statvfs`, `mount`/`umount`, and
  `klogctl` have been cross-checked against current man pages via web
  search (not just written from memory); everything else in this
  category should still be treated as reviewed-but-unverified until
  it's been through a real `cargo build` + `cargo test` on target.

The one exception: `fuzz/` (see "Fuzz testing" below) is its own
separate Cargo workspace, specifically so `libfuzzer-sys` -- which
cargo-fuzz fundamentally requires -- never becomes a dependency of
`mitos-utils` itself or of any of the ~50 shipped binaries. A plain
`cargo build`/`cargo test` at the repo root never sees `fuzz/`'s
`Cargo.toml` at all.

## Fuzz testing

`fuzz/fuzz_targets/` has four targets aimed at the hand-written
parsers most likely to have input-dependent panics that hand-written
unit tests miss: `printf`'s format renderer, `tr`'s
translate/delete logic, `cut`'s field-list range parser, and
`chmod`'s octal/symbolic mode parser. See fuzz/README.md for how to
run them (requires nightly Rust + `cargo-fuzz`, not currently wired
into `.github/workflows/ci.yml`) and each target file's own doc
comment for what property it checks.

## Directory layout

```
src/
  lib.rs           # pub mod common; pub mod applets;
  common/
    mod.rs
    errors.rs      # AppError, AppResult, run() -- now also handles --help/--version
    output.rs      # error(), error_path(), confirm(), human_size(), columnate(), reset_sigpipe()
    paths.rs       # basename(), dirname(), normalize(), walk(), walk_post_order()
    permissions.rs # format_mode(), file_type_char(), mode_of(), parse_mode()
    users.rs       # current_identity(), name_for_uid/gid(), uid/gid_for_name(), supplementary_groups()
    args.rs        # split_dashdash() -- POSIX -- handling
    safewalk.rs     # SafeDir, remove_tree/chmod_tree/chown_tree -- TOCTOU-safe recursion (Linux only)
  applets/
    mod.rs         # module declarations + the APPLETS dispatch table
    <one file per utility, pub fn run(args) + pub const USAGE>
  bin/
    <one five-line wrapper per utility, calling into applets::>
    mitos-box.rs   # the multiplexed dispatcher binary
fuzz/              # separate Cargo workspace -- see "Fuzz testing"
  Cargo.toml
  fuzz_targets/
tests/
  filesystem.rs    # mkdir/touch/cat/cp/mv/rm/ln/pwd/stat
  text.rs          # echo/printf/head/tail/grep/sort/uniq/wc/cut/tr/diff
  process.rs       # sleep/uname/hostname/env/printenv/whoami/id/groups/true
  compatibility.rs # POSIX exit-code and basename/dirname edge-case checks
  cli_infra.rs     # --help/--version/--, and mitos-box dispatch
docs/
  architecture.md  # this file
  compatibility.md # what's implemented vs. deliberately out of scope, per utility
  commands.md      # one-paragraph usage reference per utility
  integration.md   # the callable API surface, for other MITOS crates
.github/workflows/
  ci.yml           # build/clippy/test/binary-size-budget on push/PR
```

## The applet shape every utility follows

```rust
// src/applets/cat.rs
use mitos_utils::common::errors::{AppError, AppResult};

pub const USAGE: &str = "cat [-n|-b] [FILE...] -- concatenate files to stdout";

pub fn run(args: Vec<String>) -> AppResult<()> {
    // ... do the work, using `?` on anything that returns AppResult
    // or implements `From<std::io::Error> for AppError` ...
    Ok(())
}
```

```rust
// src/bin/cat.rs
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("cat", mitos_utils::applets::cat::USAGE, args, mitos_utils::applets::cat::run)
}
```

Utilities that loop over several arguments (most of them: `cat`,
`cp`, `rm`, `chmod`, ...) print a per-item error as they go via
`common::output::error_path` and track a `had_error` flag, returning
`Err(AppError::silent(1))` at the end so `run()` sets the right exit
code without printing a duplicate summary message -- matching how
GNU coreutils report "some of N files failed."
