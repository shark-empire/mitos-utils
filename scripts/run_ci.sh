#!/bin/bash
set -e

echo "==> Checking formatting..."
cargo fmt --all -- --check

echo "==> Building all targets..."
cargo build --all-targets --verbose

echo "==> Running tests..."
cargo test --verbose

echo "==> Clippy linting..."
cargo clippy --all-targets -- -D warnings

echo "All CI steps passed!"
