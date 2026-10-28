#!/bin/sh
set -eu
report=${WASM_CI_REPORT_DIR:-output/wasm-ci}
mkdir -p "$report"
export WASM_BUILD_TIMINGS="$PWD/$report/bindings.tsv"
: > "$report/timings.tsv"
: > "$WASM_BUILD_TIMINGS"
measure() {
    name=$1
    shift
    /usr/bin/time -f "$name\t%e\t%U\t%S\t%M" -a -o "$report/timings.tsv" "$@"
}
{ rustc -vV; cargo -V; wasm-bindgen --version; sccache --version; node --version; chromium --version; uname -a; nproc; git -c safe.directory="$PWD" rev-parse HEAD; } > "$report/environment.txt"
target=${CARGO_TARGET_DIR:-/workspace/target}
export CARGO_TARGET_DIR="$target/shared"
measure native cargo build --locked --release --bin planorama
measure browser-build sh scripts/build-wasm.sh
measure browser-memory node tests/browser/memory.cjs output/browser/pkg
measure browser-tests node tests/browser/wasm.cjs
measure standalone-lib cargo test --locked --no-default-features --lib
measure standalone-cli cargo test --locked --no-default-features --test cli help_
measure standalone-native cargo build --locked --release --bin planorama --no-default-features
measure standalone-build sh scripts/build-wasm-engine.sh
measure standalone-memory node tests/browser/memory.cjs
measure standalone-tests node tests/browser/wasm-engine.cjs
sccache --show-stats > "$report/sccache.txt"
du -sk /usr/local/cargo/registry /usr/local/cargo/git "$target" > "$report/cache-sizes.txt"
