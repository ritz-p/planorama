#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
case ${1:-json} in json|html) ;; *) echo 'Expected json or html' >&2; exit 2;; esac
if [ "$(cargo llvm-cov --version)" != 'cargo-llvm-cov 0.6.21' ]; then
    echo 'Install cargo-llvm-cov 0.6.21 before measuring coverage.' >&2
    exit 1
fi
out=output/rust-coverage
mkdir -p "$out"
exclude='(^|/)(tests|examples)/'
cargo llvm-cov --locked --all-features --all-targets --ignore-filename-regex "$exclude" \
    --json --output-path "$out/coverage.json"
if [ "${1:-json}" = html ]; then
    cargo llvm-cov report --html --ignore-filename-regex "$exclude" --output-dir "$out"
fi
