# Compatibility notes

`mitos-utils` targets the commonly-used subset of POSIX/GNU coreutils
behavior for each tool, not full flag-for-flag parity. This page
records what's covered and what's deliberately left out, so nobody
has to rediscover it by reading source. If a script needs a flag not
listed here as "supported", assume it isn't implemented yet.

## Cross-cutting behavior (applies to every applet)

- **`--help`**: prints a one-line usage string and exits `0`, before
  the applet's own argument parsing runs. Deliberately *not* `-h` --
  see docs/architecture.md for why (`ls -h`/`du -h` already mean
  "human-readable").
- **`--version`**: prints `"<name> (mitos-utils <version>)"` and
  exits `0`.
- **`--`**: end-of-options marker, supported by every applet that
  takes file/path positional arguments (so e.g. `rm -- -oddfile`
  works) -- see `common::args::split_dashdash` and
  `tests/cli_infra.rs`. Applets with no flag/positional ambiguity in
  the first place (`pwd`, `true`, `sync`, ...) don't need it and
  don't call it.
- **`mitos-box`**: every applet is also reachable through the single
  multiplexed binary (`mitos-box <name> ...`, or a symlink named
  `<name>` pointing at it) -- see docs/architecture.md.

## Platform scope

This crate builds `std`-hosted binaries, not freestanding kernel
code -- see docs/architecture.md for why that's the current state of
the mitosOS<->mitos-utils relationship. A few consequences:

- `chmod`, `chown`, `chgrp`, `ln -s`, `mount`, `umount`, `sync`,
  `dmesg`, `kill` are **unix-only** by construction (they wrap
  POSIX/Linux-specific syscalls). They will not compile on a
  non-unix `cfg` target as written.
- `ps`, `free`, `uptime`, `dmesg` are gated on
  `cfg(target_os = "linux")` specifically, because they read `/proc`
  or call Linux-only syscalls (`klogctl`). On any other target they
  return a clear "not available on this target" error rather than
  guessing or silently returning zeroes.
- `df`'s `statvfs` FFI binding assumes glibc's Linux/x86_64 struct
  layout. It is not expected to be correct on musl or BSD/macOS
  (different field layout) and is gated to `target_os = "linux"`
  accordingly. This layout, along with `mount`/`umount`'s signatures
  and `dmesg`'s `klogctl` signature, has been cross-checked against
  the real `statvfs(3)`/`mount(2)`/`syslog(2)` man pages (not just
  written from memory) -- see the "FFI verification" note below.
- `dmesg` additionally needs `/proc/sys/kernel/dmesg_restrict` to be
  `0` (or the caller to have `CAP_SYSLOG`/`CAP_SYS_ADMIN`) on modern
  kernels; a permission error here is expected sandboxed/unprivileged
  behavior, not a bug.
- **`rm -r`/`chmod -R`/`chown -R`/`chgrp -R`'s TOCTOU hardening
  (`common::safewalk`) is Linux-only.** On other targets, recursive
  operations fall back to the original plain path-based walk (no
  symlink-race protection). This isn't a struct-layout risk like the
  items above -- it's a deliberate scope decision, since the
  hardening's design specifically avoids the one thing (`O_DIRECTORY`/
  `O_NOFOLLOW`'s per-architecture flag values) that would have made
  extending it risky. See `safewalk.rs`'s own doc comment.
- None of this has been compiled with a real Rust toolchain (none
  was available while writing it -- see docs/architecture.md). The
  logic has been manually reviewed (brace/paren balance, symbol
  cross-referencing, and -- for the riskiest FFI structs and flag
  values -- diffed against real man pages/kernel headers) but should
  be treated as unverified until it's been through `cargo build` and
  `cargo test` for real.

## FFI verification

The hand-written `extern "C"` bindings are the highest-risk code in
this crate (a wrong struct layout is undefined behavior, not just a
wrong answer, per docs/architecture.md's "zero dependencies"
rationale for why they exist at all instead of using the `libc`
crate). Cross-checked field-for-field against current man pages:

- `statvfs` (`df`): field order/types match `statvfs(3)` exactly
  (`f_bsize`, `f_frsize`, `f_blocks`, `f_bfree`, `f_bavail`,
  `f_files`, `f_ffree`, `f_favail`, `f_fsid`, `f_flag`, `f_namemax`,
  plus glibc's trailing `int[6]` reserved padding).
- `mount`/`umount` (`mount`, `umount`): signatures match `mount(2)`
  exactly (`mount(source, target, filesystemtype, mountflags, data)`,
  `umount(target)`).
- `klogctl` (`dmesg`): signature and the `SYSLOG_ACTION_READ_ALL = 3`
  constant match `syslog(2)`'s glibc wrapper exactly.
- `AT_SYMLINK_NOFOLLOW = 0x100` / `AT_REMOVEDIR = 0x200`
  (`common::safewalk`): confirmed identical across every Linux
  architecture, including arm64, via `include/uapi/linux/fcntl.h` and
  independent secondary sources.
- **Explicitly avoided rather than verified**: `O_DIRECTORY`/
  `O_NOFOLLOW`. These are *not* architecture-independent -- glibc/
  x86_64 defines `O_DIRECTORY = 0200000`, but the arm64 kernel's own
  uapi headers define `O_DIRECTORY = 040000` for AArch32-compat
  reasons, and a real 2024 QEMU bug report shows `O_NOFOLLOW` arriving
  as `O_LARGEFILE` under exactly this kind of cross-arch confusion.
  `common::safewalk` was designed specifically to need neither value
  (see that file's doc comment for the check-open-verify pattern used
  instead) rather than risk hand-rolling a constant that's wrong on
  one of mitosOS's two target architectures with no build error to
  catch it.

Not yet independently cross-checked against a header/man page (lower
risk -- simpler signatures, well-established POSIX calls, but still
unverified by an actual compile): `getpwuid`/`getpwnam`/
`getgrgid`/`getgrnam`/`getgroups` (`common::users`), `chown`, `kill`,
`sync`, `uname`, `gethostname`, `openat`/`unlinkat`/`fchmodat`/
`fchownat` (`common::safewalk` -- these four have simple
pointer/integer signatures with no struct-layout or cross-arch
flag-value risk, unlike `O_DIRECTORY`/`O_NOFOLLOW` above).

## Known cross-cutting limitation: binary-safety

The text utilities (`cat`, `head`, `tail`, `grep`, `sort`, `uniq`,
`wc`, `cut`, `tr`, `tee`) operate on UTF-8 text via `str`/`String` and
line-oriented reads. They are not binary-safe the way GNU's versions
are (which operate on raw bytes and handle invalid UTF-8 and NUL
bytes gracefully). Running them on arbitrary binary files may behave
oddly or error rather than passing bytes through unchanged.

## Per-utility notes

### Filesystem

| Tool | Supported | Not implemented |
|---|---|---|
| `cat` | `-n`, `-b`, multiple files, `-` for stdin | binary-safe passthrough, `-A`/`-s`/`-T` |
| `ls` | `-a`, `-l`, `-h`, multiple paths | sorting flags (`-t`, `-S`), `-R`, color, `-i` |
| `mkdir` | `-p` | `-m MODE` |
| `rmdir` | plain removal | `-p` (remove ancestors), `--ignore-fail-on-non-empty` |
| `touch` | create-if-missing, update mtime | `-t TIMESTAMP`, `-r REFFILE`, `-a`/`-m` (atime only) |
| `cp` | `-r`/`-R`/`--recursive`, `-i`, `-p` (mtime only, see note below) | `-v`, symlink handling flags; `-p` doesn't preserve nested subdirectory mtimes in a `-r` tree (see `cp.rs`), and doesn't preserve ownership (needs root) |
| `mv` | rename, cross-filesystem fallback (preserves mtime in the fallback), `-i` | `-v`, `-n` |
| `rm` | `-r`/`-R`, `-f`, `-i`, `--` | `-v`; `-i` prompts once per top-level argument, not once per file inside a `-r` tree; TOCTOU-hardened recursion is Linux-only (see "Platform scope" above) |
| `ln` | `-s`, `-f` | `-v`, `-b` (backup), hard-link-of-directory guard |
| `pwd` | plain | `-L`/`-P` distinction (always logical) |
| `basename` | suffix stripping, POSIX edge cases | `-a` (multiple operands), `-s` |
| `dirname` | POSIX edge cases | -- |
| `realpath` | symlink resolution, `-m`-style lexical fallback | `-s` (no-symlink), `--relative-to` |
| `readlink` | plain, `-f` | `-e`/`-m` distinction (folded into `-f`) |
| `stat` | type, size, mode, owner/group, mtime | `--format`, block-device-specific fields, atime/ctime |

### Text processing

| Tool | Supported | Not implemented |
|---|---|---|
| `echo` | `-n`, `-e` | `-E` (explicit no-escape; it's the default) |
| `printf` | `%s`, `%d`, `%x`, `%%`, `\n`/`\t`/`\\` | width/precision (`%5d`, `%.2f`), `%f`, `%o`, `%c` |
| `head` | `-n N`, `-N`, `-c N` (bytes), multiple files | `-q`/`-v` header control |
| `tail` | `-n N`, `-N`, `-c N` (bytes; seeks on real files, buffers on stdin), multiple files | `-f` (follow) |
| `grep` | substring match, `-i`, `-v`, `-n`, multiple files | regular expressions (this is closer to `grep -F`) |
| `sort` | `-r`, `-n`, `-u`, `-k N` (single field, not an `N,M` range), `-t DELIM` | multiple/composite sort keys, stable-sort guarantee across ties beyond Rust's stable `sort_by` |
| `uniq` | adjacent-duplicate collapsing, `-c`, `-d` | `-u` (unique-only), `-i` (case-insensitive) |
| `wc` | `-l`, `-w`, `-c`, multi-file totals; streams in fixed-size chunks so it stays cheap on very large files | `-m` (character count, distinct from byte count under multi-byte encodings) |
| `cut` | `-d`, `-f` with ranges/lists | `-c` (character ranges), `-b` (byte ranges) |
| `tr` | literal set translation, `-d` | `[a-z]`-style ranges, `-c` (complement), `-s` (squeeze) |
| `tee` | `-a`/`--append`, multiple files | `-i` (ignore SIGINT) |
| `diff` | line-by-line change/add/delete report | true LCS-based diff (this can over-report changes for interleaved edits vs. `diff`'s minimal edit script), `-u` unified format, `-r` (recursive) |

### Process / system

| Tool | Supported | Not implemented |
|---|---|---|
| `ps` | pid + state + command, Linux `/proc` only | full field set (`%CPU`, `%MEM`, TTY, elapsed time), filtering flags |
| `kill` | `-9`/`-KILL`, `-15`/`-TERM`, `-1`/`-HUP`, `-2`/`-INT`, arbitrary `-N` | signal names beyond the four listed, `-l` (list signals) |
| `sleep` | plain seconds, `s`/`m`/`h` suffixes | `d` (days) suffix, fractional-with-suffix combos |
| `uptime` | uptime duration, Linux `/proc/uptime` only | load averages, logged-in user count |
| `free` | total/used/free/available, Mem + Swap, Linux only | `-h` is implicit (always human-readable); no raw-byte mode; no `-s` (repeat) |
| `uname` | `-a`, `-s`, `-n`, `-r`, `-m` | `-v`, `-o`, `-p` |
| `hostname` | print only | setting the hostname (`hostname NEWNAME`) |
| `env`/`printenv` | list all, look up by name | `env -i`/`env VAR=val CMD` (running a command with a modified environment) |
| `whoami` | plain | -- |
| `id` | `uid=`/`gid=`/`groups=` line | `-u`/`-g`/`-G`/`-n` (individual-field modes) |
| `groups` | current user only | `groups USERNAME` (another user) |

### Disk / mount

| Tool | Supported | Not implemented |
|---|---|---|
| `df` | size/used/avail per path, Linux/glibc only | `-h` is implicit; no `-i` (inode counts); no "Filesystem" source column (statvfs doesn't expose the device name) |
| `du` | recursive size sum, `-h`, `-s` | `-a` (per-file, not just per-directory), `--max-depth`, hard-link double-counting isn't deduplicated |
| `mount` | list (`/proc/mounts`, Linux only), basic `SOURCE TARGET -t FSTYPE` | mount options (`-o`), bind mounts, remount |
| `umount` | plain target unmount | `-f` (force), `-l` (lazy) |
| `sync` | full sync via `sync(2)` | per-file sync (`syncfs`, `fsync` on a specific fd) |
| `dmesg` | full buffer dump, Linux only | `-c` (clear after read), `-w` (follow), timestamp formatting/filtering |

### Permissions

| Tool | Supported | Not implemented |
|---|---|---|
| `chmod` | octal (`755`), symbolic (`u+x`, `go-w`, `a=r,u+w`), `-R` (TOCTOU-hardened on Linux -- see "Platform scope"), `--` | `X` (conditional execute), `s`/`t` symbolic bits, `--reference` |
| `chown` | `owner`, `owner:group`, `-R` (TOCTOU-hardened on Linux), `--` | numeric uid:gid without a passwd entry, `--reference` |
| `chgrp` | group name, `-R` (TOCTOU-hardened on Linux), `--` | numeric gid without a group entry, `--reference` |

### Misc

| Tool | Supported | Not implemented |
|---|---|---|
| `clear` | ANSI clear-screen escape | terminfo-driven clearing (correct on virtually all modern terminals anyway) |
| `true` | exits 0 | -- (`false` isn't in the framework's list, so it wasn't added) |
