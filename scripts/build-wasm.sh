#!/bin/sh
set -eu
. scripts/wasm-build-timing.sh
test "$(wasm-bindgen --version)" = 'wasm-bindgen 0.2.100'
wasm_step browser-compile cargo build --locked --release --lib --target wasm32-unknown-unknown
mkdir -p output/browser/pkg
wasm_step browser-bindings wasm-bindgen --target no-modules --no-typescript --out-dir output/browser/pkg "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/planorama.wasm"
node scripts/prepare-wasm-memory.cjs output/browser/pkg
cp browser/index.html browser/app.js browser/worker.js output/browser/
node scripts/build-spatial-options.cjs
