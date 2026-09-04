# mitos-utils

Core system utilities for [MITOS](../../mitosos) -- a coreutils-style
suite of ~50 small userspace programs (`cat`, `ls`, `grep`, `chmod`,
`ps`, ...), each one both a standalone binary *and* a plain callable
Rust function, sharing a small common library for error handling,
path logic, permission formatting, user/group lookups, and
TOCTOU-safe recursive directory operations. Also buildable as a
single multiplexed binary (`mitos-box`, busybox/toybox-style) instead
of ~50 separate binaries.

## Status

Written and reviewed against a hosted Unix target (Linux) with the
highest-risk FFI (`statvfs`, `mount`/`umount`, `klogctl`, and the
TOCTOU-hardening flag values) cross-checked against real man
pages/kernel headers via web search -- but not yet compiled (no Rust
toolchain was available while writing it) or run against mitosOS
itself, which doesn't have a userspace to host this crate on yet.
`.github/workflows/ci.yml` will be the first environment that
actually compiles it. See `docs/architecture.md` for what that means
and what's next.

## Status

Written and reviewed against a hosted Unix target (Linux) with the
highest-risk FFI (`statvfs`, `mount`/`umount`, `klogctl`, and the
TOCTOU-hardening flag values) cross-checked against real man
pages/kernel headers via web search -- but not yet compiled (no Rust
toolchain was available while writing it) or run against mitosOS
itself, which doesn't have a userspace to host this crate on yet.
`.github/workflows/ci.yml` will be the first environment that
actually compiles it. See `docs/architecture.md` for what that means
and what's next.

## What's done vs. what's left

A living checklist -- update it as items get crossed off for real
(compiled, run, confirmed), not just written.

### Done

- [x] All 50 utilities + `common` library (errors, output, paths,
      permissions, users)
- [x] Zero dependencies in the main crate (`fuzz/` is the one
      deliberate, isolated exception)
- [x] `--help` / `--version` on every command
- [x] POSIX `--` end-of-options on every command that takes file/path
      arguments
- [x] TOCTOU-hardened `rm -r` / `chmod -R` / `chown -R` / `chgrp -R`
      (Linux)
- [x] Single multiplexed binary (`mitos-box`) dispatching to all 50
      applets
- [x] `ls -R` / `-t` / `-S`
- [x] Streaming `wc` (fixed-size chunks, not whole-file reads)
- [x] `-i` (interactive confirm) on `rm`/`cp`/`mv`; `-p` (preserve
      mtime) on `cp`
- [x] CI pipeline: build, clippy, test, binary-size budget
      (`.github/workflows/ci.yml`) -- **written, not yet run**
- [x] Fuzz test scaffolding: 4 targets (`printf`, `tr`, `cut`'s field
      parser, `chmod`'s mode parser) -- **written, not yet run**
- [x] Real `man` pages: all 50 utilities + `mitos-box(1)` + 3
      overview pages (`man/man1/`, `man/man7/`)
- [x] Integration API reference for other MITOS crates
      (`docs/integration.md`)

### Known limitations (by design, not bugs)

- FFI cross-checked against real man pages/headers: `statvfs`,
  `mount`/`umount`, `klogctl`, the `AT_*` flags used by the TOCTOU
  hardening. **Not yet independently verified**: the `getpwuid`
  family, `chown`, `kill`, `sync`, `uname`, `gethostname`,
  `openat`/`unlinkat`/`fchmodat`/`fchownat` (lower risk -- simple
  signatures, no struct-layout exposure, but still unconfirmed by an
  actual compile).
- TOCTOU hardening (`common::safewalk`) is **Linux-only**; other
  targets fall back to an unhardened path-based walk.
- Text tools are **not binary-safe** (UTF-8 line-oriented, not raw
  bytes).
- **No locale support** -- C-locale/byte-order behavior only; no
  `LC_COLLATE`-aware sorting, no multi-byte-aware character counting.
- `grep` is **substring-only**, no regular expressions.
- `sort -k` supports a **single field**, not an `N,M` range.
- `cp -p` doesn't preserve **nested subdirectory** mtimes inside a
  `-r` tree, only the root and individual files.
- No extended attributes / ACLs -- also arguably premature: mitosOS's
  filesystem is FAT32 today, which has no concept of either for these
  tools to preserve.

### Not started

- [ ] **Compiled and run, even once.** The single biggest gap --
      everything above is reviewed, not verified. First real signal
      will be the CI pipeline's first run.
- [ ] Regex support in `grep` (a real engine, not a small tweak --
      no external dependency currently planned for it, given the
      zero-dependency stance)
- [ ] Locale-aware collation
- [ ] Shell completions (bash/zsh/fish)
- [ ] Windows support -- not attempted, and arguably not a real goal:
      mitosOS is a POSIX-style kernel, and most of this crate's
      utilities (`chmod`, `chown`, `ln -s`, `mount`, ...) are
      inherently Unix concepts. The text-processing tools would
      likely compile on Windows as-is if that ever became useful.
- [ ] Fuzz targets actually run (needs nightly Rust + `cargo-fuzz`
      locally; see `fuzz/README.md`)

## Layout

- `src/common/` -- shared library code (errors, output formatting,
  path logic, permission formatting, user/group lookups, `--`
  handling, TOCTOU-safe recursion). See `docs/architecture.md`.
- `src/applets/` -- every utility's actual logic, as a callable
  `run(args)` function.
- `src/bin/` -- a thin wrapper binary per utility, plus `mitos-box`
  (the multiplexed dispatcher).
- `man/` -- real troff `man` pages: `man/man1/<name>.1` for every
  utility plus `mitos-box`, `man/man7/` for the three overview pages
  cross-referenced from all of them (`mitos-utils-compat(7)`,
  `mitos-utils-security(7)`, `mitos-utils-integration(7)`).
- `fuzz/` -- a separate Cargo workspace with fuzz targets for the
  hand-written parsers most likely to have input-dependent panics.
  See `fuzz/README.md`.
- `tests/` -- integration tests that run the compiled binaries.
- `docs/` -- architecture notes, a full command reference
  (`commands.md`), a per-utility compatibility matrix
  (`compatibility.md`), and an API reference for integrating this
  crate into another MITOS project (`integration.md`).

## Building

```sh
cargo build --release
```

Each utility in `src/bin/` becomes its own binary under
`target/release/`, including `mitos-box`. The main crate has zero
external dependencies (see `docs/architecture.md` for why, and for
the one deliberate exception in `fuzz/`).

## Testing

```sh
cargo test
```

## License

MIT -- see `LICENSE`.
