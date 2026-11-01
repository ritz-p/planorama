#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-wasm-bindings.sh output/browser/pkg output/wasm-engine
sh scripts/assemble-browser.sh
node scripts/package-wasm-engine.cjs
