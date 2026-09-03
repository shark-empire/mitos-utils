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

## Layout

- `src/common/` -- shared library code (errors, output formatting,
  path logic, permission formatting, user/group lookups, `--`
  handling, TOCTOU-safe recursion). See `docs/architecture.md`.
- `src/applets/` -- every utility's actual logic, as a callable
  `run(args)` function.
- `src/bin/` -- a thin wrapper binary per utility, plus `mitos-box`
  (the multiplexed dispatcher).
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
