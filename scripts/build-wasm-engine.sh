#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-wasm-bindings.sh output/wasm-engine
node scripts/package-wasm-engine.cjs
