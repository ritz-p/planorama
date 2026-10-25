#!/bin/sh
set -eu
mkdir -p output/wasm-ci
measure() {
    name=$1
    shift
    /usr/bin/time -f "$name\t%e\t%U\t%S\t%M" -a -o output/wasm-ci/timings.tsv "$@"
}
{ rustc -vV; cargo -V; wasm-bindgen --version; sccache --version; uname -a; nproc; } > output/wasm-ci/environment.txt
target=${CARGO_TARGET_DIR:-/workspace/target}
export CARGO_TARGET_DIR="$target/default"
measure native cargo build --locked --release --bin planorama
measure browser-build sh scripts/build-wasm.sh
measure browser-memory node tests/browser/memory.cjs output/browser/pkg
measure browser-tests node tests/browser/wasm.cjs
export CARGO_TARGET_DIR="$target/standalone"
measure standalone-lib cargo test --locked --no-default-features --lib
measure standalone-cli cargo test --locked --no-default-features --test cli help_
measure standalone-native cargo build --locked --release --bin planorama --no-default-features
measure standalone-build sh scripts/build-wasm-engine.sh
measure standalone-memory node tests/browser/memory.cjs
measure standalone-tests node tests/browser/wasm-engine.cjs
sccache --show-stats > output/wasm-ci/sccache.txt
du -sk /usr/local/cargo/registry /usr/local/cargo/git /workspace/target > output/wasm-ci/cache-sizes.txt
