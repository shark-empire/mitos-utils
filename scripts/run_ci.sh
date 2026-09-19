#!/bin/bash
# Mirrors most of .github/workflows/ci.yml for a fast local check
# before pushing. Skips the slower/extra-setup jobs (msrv, audit,
# fuzz-build, binary-size) -- see the printed commands below to run
# those too.
set -e

echo "==> Checking formatting..."
cargo fmt --all -- --check

echo "==> Clippy linting..."
cargo clippy --all-targets -- -D warnings

echo "==> Building all targets..."
cargo build --all-targets --verbose

echo "==> Running tests..."
cargo test --verbose

echo "==> Building docs (deny warnings)..."
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

echo "All core CI checks passed!"
echo ""
echo "Not run by this script (see .github/workflows/ci.yml):"
echo "  cargo +1.74.0 build --all-targets        # MSRV check"
echo "  (cd fuzz && cargo +nightly check)        # fuzz targets still compile"
echo "  cargo generate-lockfile && cargo audit   # needs cargo-audit installed"
echo "  cargo build --release --bins             # for the binary-size budget"
