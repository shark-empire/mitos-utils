# Compatibility notes

`mitos-utils` targets the commonly-used subset of POSIX/GNU coreutils
behavior for each tool, not full flag-for-flag parity. This page
records what's covered and what's deliberately left out, so nobody
has to rediscover it by reading source. If a script needs a flag not
listed here as "supported", assume it isn't implemented yet.

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
  accordingly.
- None of this has been compiled with a real Rust toolchain (none
  was available while writing it -- see docs/architecture.md). The
  logic has been manually reviewed (brace/paren balance, symbol
  cross-referencing) but should be treated as unverified until it's
  been through `cargo build` and `cargo test` for real.

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
| `cp` | `-r`/`-R`/`--recursive` | `-p` (preserve mode/owner/time), `-i`, `-v`, symlink handling flags |
| `mv` | rename, cross-filesystem fallback | `-i`, `-v`, `-n` |
| `rm` | `-r`/`-R`, `-f` | `-i`, `-v` |
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
| `head` | `-n N`, `-N`, multiple files | `-c` (byte count), `-q`/`-v` header control |
| `tail` | `-n N`, `-N`, multiple files | `-f` (follow), `-c` |
| `grep` | substring match, `-i`, `-v`, `-n`, multiple files | regular expressions (this is closer to `grep -F`) |
| `sort` | `-r`, `-n`, `-u` | `-k` (key field), `-t` (field separator), stable-sort guarantee across ties beyond Rust's stable `sort_by` |
| `uniq` | adjacent-duplicate collapsing, `-c`, `-d` | `-u` (unique-only), `-i` (case-insensitive) |
| `wc` | `-l`, `-w`, `-c`, multi-file totals | `-m` (character count, distinct from byte count under multi-byte encodings) |
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
| `chmod` | octal (`755`), symbolic (`u+x`, `go-w`, `a=r,u+w`), `-R` | `X` (conditional execute), `s`/`t` symbolic bits, `--reference` |
| `chown` | `owner`, `owner:group`, `-R` | numeric uid:gid without a passwd entry, `--reference` |
| `chgrp` | group name, `-R` | numeric gid without a group entry, `--reference` |

### Misc

| Tool | Supported | Not implemented |
|---|---|---|
| `clear` | ANSI clear-screen escape | terminfo-driven clearing (correct on virtually all modern terminals anyway) |
| `true` | exits 0 | -- (`false` isn't in the framework's list, so it wasn't added) |
