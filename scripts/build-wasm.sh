#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
sh scripts/build-wasm-bindings.sh output/browser/pkg
sh scripts/assemble-browser.sh
