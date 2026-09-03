# Integrating with other MITOS projects

mitos-utils is usable two ways by another MITOS crate: as an
**in-process library** (call a function, no subprocess involved) or
as **standalone executables** invoked as subprocesses (the
traditional coreutils model, `PATH`-resolved and spawned via
`fork`/`exec` or your platform's equivalent). Both are first-class:
every applet's logic lives in `mitos_utils::applets::<name>::run`,
and each `src/bin/<name>.rs` executable is a five-line wrapper around
that exact same function (see docs/architecture.md).

The in-process route is the more interesting one *today*: mitosOS
doesn't have `fork`/`exec` yet (see "What mitosOS needs to expose"
below), so calling straight into `mitos_utils::applets::*` is
currently the only way a fellow hosted-std MITOS project (namely
[[mitos-shell]]) could use mitos-utils' logic at all.

## Depending on mitos-utils as a library

```toml
# in another MITOS crate's Cargo.toml
[dependencies]
mitos-utils = { path = "../mitos-utils" }   # or a git dependency
```

Then, e.g., to run `cat` on a file without spawning a process:

```rust
use mitos_utils::applets;

let args = vec!["myfile.txt".to_string()];
match applets::cat::run(args) {
    Ok(()) => { /* wrote to stdout already */ }
    Err(e) => eprintln!("cat: {}", e),
}
```

## `common::` -- shared building blocks

Everything below is `pub`, unconditionally usable by any crate that
depends on mitos-utils (items marked *unix* need `cfg(unix)`; items
marked *Linux* need `cfg(target_os = "linux")` -- see
docs/compatibility.md for why).

| Module | Item | Signature | What it's for |
|---|---|---|---|
| `errors` | `AppError` | struct: `{ message: String, code: u8 }` | The error type every applet returns |
| `errors` | `AppError::new/usage/with_code/silent` | `(impl Into<String>[, u8]) -> AppError` | Construct an error with the right exit code |
| `errors` | `AppResult<T>` | `type AppResult<T> = Result<T, AppError>` | Every applet's `run` returns `AppResult<()>` |
| `errors` | `EXIT_SUCCESS`/`EXIT_FAILURE`/`EXIT_USAGE` | `u8` consts (`0`/`1`/`2`) | Exit-code conventions, POSIX/GNU-style |
| `errors` | `run` | `(prog: &str, usage: &str, args: Vec<String>, body: impl FnOnce(Vec<String>) -> AppResult<()>) -> ExitCode` | The shared entry point: handles `--help`/`--version`, calls `body`, reports errors. What every `src/bin/*.rs` and `mitos-box` call |
| `output` | `error`/`error_path` | `(prog: &str, ...)` | Consistent `"prog: msg"` / `"prog: path: msg"` stderr formatting |
| `output` | `confirm` | `(prompt: &str) -> bool` | The `-i` yes/no prompt used by `rm`/`cp`/`mv` |
| `output` | `human_size` | `(bytes: u64) -> String` | `ls -h`/`du -h`/`df`/`free`-style byte formatting |
| `output` | `columnate` | `(names: &[String], term_width: usize)` | `ls`'s multi-column layout, printed directly |
| `output` | `reset_sigpipe` | *unix* `()` | Called by `errors::run`; exposed in case a caller bypasses `run` |
| `paths` | `basename`/`dirname` | `(path: &str) -> String` | POSIX-edge-case-correct path splitting |
| `paths` | `normalize` | `(path: &Path, cwd: &Path) -> PathBuf` | Lexical `.`/`..` resolution without requiring the path to exist |
| `paths` | `walk`/`walk_post_order` | `(root: &Path) -> io::Result<Vec<PathBuf>>` | Recursive directory listing, pre-/post-order |
| `permissions` | `format_mode` | `(mode: u32, file_type_char: char) -> String` | Renders e.g. `-rw-r--r--` |
| `permissions` | `file_type_char` | *unix* `(meta: &Metadata) -> char` | The `ls -l`/`stat` column-1 marker |
| `permissions` | `mode_of` | *unix* `(meta: &Metadata) -> u32` | Mode bits from `Metadata` |
| `permissions` | `parse_mode` | `(spec: &str, current: u32) -> Result<u32, String>` | `chmod`'s octal/symbolic mode parser -- reusable by anything else that needs to interpret a mode string (e.g. a future file-manager component of [[mitos-gui]]) |
| `users` | `Identity` | struct: `{ uid, gid, euid, egid: u32, user, group: String }` | Current-process identity |
| `users` | `current_identity` | `() -> Identity` | |
| `users` | `name_for_uid`/`name_for_gid` | `(u32) -> Option<String>` | id -> name lookups |
| `users` | `uid_for_name`/`gid_for_name` | `(&str) -> Option<u32>` | name -> id lookups (`chown`/`chgrp`) |
| `users` | `supplementary_groups` | `() -> Vec<u32>` | |
| `args` | `split_dashdash` | `(args: Vec<String>) -> (Vec<String>, Vec<String>)` | POSIX `--` end-of-options splitting |
| `safewalk` | `SafeDir` | *Linux* struct + methods (`open_root`, `open_subdir`, `list_names`, `entry_is_symlink`, `entry_mode`, `remove_file`, `remove_empty_dir`, `chmod_entry`, `chown_entry`) | TOCTOU-safe directory traversal primitive -- see the module's own doc comment for the full design rationale |
| `safewalk` | `remove_tree`/`chmod_tree`/`chown_tree` | *Linux* `(root: &Path, ...) -> io::Result<()>` | The three hardened recursive operations built on `SafeDir`, usable directly by anything else that needs a TOCTOU-safe `rm -r`/`chmod -R`/`chown -R` (rather than reimplementing the pattern) |

## `applets::` -- every utility as a function

`applets::APPLETS` is a `&[(&str, &str, AppletFn)]` -- `(name, usage,
run)` for all 50, in the order below. `AppletFn` is
`fn(Vec<String>) -> AppResult<()>`. Any MITOS crate can either call
`applets::<name>::run(args)` directly (knowing the name at compile
time) or walk `APPLETS` to dispatch by a name known only at runtime
-- which is exactly what `mitos-box` (`src/bin/mitos-box.rs`) does,
and is the pattern [[mitos-shell]]'s executor could use too (see
below).

`cat`, `ls`, `mkdir`, `rmdir`, `touch`, `cp`, `mv`, `rm`, `ln`, `pwd`,
`basename`, `dirname`, `realpath`, `readlink`, `stat`, `echo`,
`printf`, `head`, `tail`, `grep`, `sort`, `uniq`, `wc`, `cut`, `tr`,
`tee`, `diff`, `ps`, `kill`, `sleep`, `uptime`, `free`, `uname`,
`hostname`, `env`, `printenv`, `whoami`, `id`, `groups`, `df`, `du`,
`mount`, `umount`, `sync`, `dmesg`, `chmod`, `chown`, `chgrp`,
`clear`, `true` -- see docs/commands.md for each one's flags.

A handful of applets also expose their inner parsing/rendering logic
as standalone `pub fn`s, one level more granular than the whole
`run`, for callers that want just the computation without the
CLI/stdio wrapper around it (this is also what the `fuzz/` targets
call -- see fuzz/README.md):

| Function | Signature | Applet |
|---|---|---|
| `applets::printf::render` | `(format: &str, args: &[String]) -> String` | `printf` |
| `applets::tr::translate` | `(input: &str, set1: &[char], set2: &[char]) -> String` | `tr` |
| `applets::tr::delete_chars` | `(input: &str, set1: &[char]) -> String` | `tr` |
| `applets::cut::parse_field_list` | `(spec: &str) -> AppResult<Vec<usize>>` | `cut` |
| `applets::echo::expand_escapes` | `(s: &str) -> String` | `echo` |

## The subprocess contract (for when it's spawned, not called)

For the eventual world where mitosOS has real process creation and a
shell spawns `mitos-box`/the standalone binaries as child processes
rather than linking them in:

- **Exit codes**: `0` success, `1` general failure, `2` bad usage --
  `common::errors::{EXIT_SUCCESS,EXIT_FAILURE,EXIT_USAGE}`. `grep`
  additionally uses `1` for "ran fine, no match" (GNU convention).
- **stdout/stderr**: normal output on stdout; every error as
  `"<prog>: <message>"` (or `"<prog>: <path>: <message>"`) on
  stderr, nothing else on stderr.
- **`--help`/`--version`**: supported uniformly, handled before an
  applet's own argument parsing ever runs (`common::errors::run`).
- **`--`**: supported by every applet that takes file/path
  arguments, via `common::args::split_dashdash`.
- **SIGPIPE**: reset to default disposition on unix at the start of
  every applet (`common::output::reset_sigpipe`), so piping into
  something that closes early exits quietly instead of panicking.

## What mitos-shell specifically could do with this

Per mitos-shell's own module layout, its `builtins/` (`alias`,
`eval`, `read`, `set`, `test`, `trap`) already cover the commands
that *must* run in the shell's own process (they change shell
state -- an external `cd` couldn't affect the parent shell's
directory). Everything in mitos-utils' `APPLETS` table is the
complementary set: commands that only touch the outside world (files,
processes, environment display) and have no reason to run in-process
*except* for the performance/no-fork-needed argument above. Two
integration shapes mitos-shell's `execution/executor.rs` could use,
depending on how much it wants mitos-utils in its dependency tree:

1. **Link mitos-utils in and call `applets::APPLETS` directly**
   (no subprocess, works today, before mitosOS has `fork`/`exec`).
2. **Spawn the standalone binaries** once mitosOS can actually create
   processes, using the subprocess contract above -- at that point
   `mitos-box` plus symlinks is the disk-cheap way to install all 50.

## What mitosOS needs to expose

The concrete, current-session list of what the kernel doesn't have
yet that various applets are written against as their target
interface (each one's own doc comment says this too; collected here
in one place):

- **`fork`/`exec`-equivalent process creation** -- needed for
  mitos-utils to be *spawned* at all (today it can only be linked in,
  per above).
- **A `/proc`-like pseudo-filesystem** -- `ps`, `free`, `uptime`
  (per-pid `comm`/`stat`, `/proc/meminfo`, `/proc/uptime`).
- **`statvfs`(-equivalent) syscall** -- `df`.
- **A mount table exposed to userspace** -- `mount` (list mode),
  reads `/proc/mounts` today.
- **A kernel log buffer read syscall** -- `dmesg` (`klogctl`-shaped
  today).
- **A passwd/group database or equivalent identity syscalls** --
  `common::users`, currently shells out to the *host* libc's
  `getpwuid`/`getpwnam`/etc., which won't exist on mitosOS itself.

## mitos-gui

No integration point today -- mitos-gui is a Wayland compositor/
desktop shell with no file-management or CLI-invocation surface yet.
If that changes (e.g. a future file-manager panel), `common::paths`,
`common::permissions`, and `applets::ls`/`applets::stat`'s logic are
the pieces most likely to be reused rather than reimplemented.
