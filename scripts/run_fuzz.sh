#!/bin/bash
# Run all mitos-utils fuzz targets locally

if ! command -v cargo-fuzz &> /dev/null; then
    echo "Installing cargo-fuzz..."
    cargo install cargo-fuzz
fi

if ! rustup toolchain list | grep -q nightly; then
    echo "Installing nightly toolchain..."
    rustup toolchain install nightly
fi

cd fuzz || exit 1

targets=("printf_render" "tr_translate" "cut_field_list" "chmod_parse_mode")

for target in "${targets[@]}"; do
    echo "==> Fuzzing $target (10s limit)..."
    cargo +nightly fuzz run "$target" -- -max_total_time=10
done
