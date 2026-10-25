#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
. scripts/wasm-build-timing.sh
test "$(wasm-bindgen --version)" = 'wasm-bindgen 0.2.100'
wasm_step standalone-compile cargo build --locked --release --lib --target wasm32-unknown-unknown --no-default-features
dependencies="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/planorama.d"
test -s "$dependencies"
if grep -E 'assets/icons/|/service-[^ ]+\.svg' "$dependencies"; then
    echo 'Artwork dependency in standalone engine' >&2
    exit 1
fi
mkdir -p output/wasm-engine
wasm_step standalone-bindings wasm-bindgen --target no-modules --no-typescript --out-dir output/wasm-engine "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/planorama.wasm"
node scripts/prepare-wasm-memory.cjs output/wasm-engine
node scripts/package-wasm-engine.cjs
