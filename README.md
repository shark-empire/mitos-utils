# mitos-utils

Core system utilities for [MITOS](../../mitosos) -- a coreutils-style
suite of ~50 small userspace programs (`cat`, `ls`, `grep`, `chmod`,
`ps`, ...) sharing a small common library for error handling, path
logic, permission formatting, and user/group lookups.

## Status

Written and reviewed against a hosted Unix target (Linux); not yet
compiled (no Rust toolchain was available while writing it) or run
against mitosOS itself, which doesn't have a userspace to host this
crate on yet. See `docs/architecture.md` for what that means and
what's next.

## Layout

- `src/common/` -- shared library code (errors, output formatting,
  paths, permissions, users). See `docs/architecture.md`.
- `src/bin/` -- one file per utility.
- `tests/` -- integration tests that run the compiled binaries.
- `docs/` -- architecture notes, a full command reference
  (`commands.md`), and a per-utility compatibility matrix
  (`compatibility.md`).

## Building

```sh
cargo build --release
```

Each utility in `src/bin/` becomes its own binary under
`target/release/`. This crate has zero external dependencies (see
`docs/architecture.md` for why).

## Testing

```sh
cargo test
```

## License

MIT -- see `LICENSE`.
