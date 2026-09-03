# Fuzzing

Requires nightly Rust and the `cargo-fuzz` subcommand
(`cargo install cargo-fuzz`), neither of which the main
`.github/workflows/ci.yml` pipeline installs -- fuzzing here is a
manual/local step for now, not (yet) part of every CI run. From the
repository root:

```sh
cargo fuzz run printf_render
cargo fuzz run tr_translate
cargo fuzz run cut_field_list
cargo fuzz run chmod_parse_mode
```

Each target's doc comment explains what property it's checking (in
every case: "never panics on adversarial input" -- none of these
targets assert anything about the *output* being correct, only that
malformed input can't crash the process). See each file under
`fuzz_targets/` for details, and docs/architecture.md for why
`libfuzzer-sys` lives in this directory's own isolated workspace
instead of mitos-utils' own `Cargo.toml`.
