#!/bin/sh
set -eu
. scripts/wasm-build-timing.sh
test "$#" -gt 0
test "$(wasm-bindgen --version)" = 'wasm-bindgen 0.2.100'
wasm_step compile cargo build --locked --release --lib --target wasm32-unknown-unknown --no-default-features
dependencies="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/planorama.d"
test -s "$dependencies"
if grep -E 'assets/icons/|/service-[^ ]+\.svg' "$dependencies"; then
    echo 'Artwork dependency in engine' >&2
    exit 1
fi
first=$1
shift
mkdir -p "$first"
wasm_step bindings wasm-bindgen --target no-modules --no-typescript --out-dir "$first" "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/planorama.wasm"
node scripts/prepare-wasm-memory.cjs "$first"
for directory in "$@"; do
    mkdir -p "$directory"
    for file in planorama.js planorama_bg.wasm memory-capabilities.json; do
        cp "$first/$file" "$directory/$file"
    done
done
