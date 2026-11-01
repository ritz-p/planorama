#!/bin/sh
set -eu
export WASM_CI_SUITE=${WASM_CI_SUITE:-full}
case $WASM_CI_SUITE in
    pr|full) ;;
    *) echo 'WASM_CI_SUITE must be pr or full' >&2; exit 2 ;;
esac
report=${WASM_CI_REPORT_DIR:-output/wasm-ci}
mkdir -p "$report"
export WASM_BUILD_TIMINGS="$PWD/$report/bindings.tsv"
export WASM_BROWSER_TIMINGS="$PWD/$report/browser.tsv"
: > "$report/timings.tsv"
: > "$WASM_BUILD_TIMINGS"
: > "$WASM_BROWSER_TIMINGS"
printf 'suite=%s\n' "$WASM_CI_SUITE" > "$report/selection.txt"
measure() {
    name=$1
    shift
    /usr/bin/time -f "$name\t%e\t%U\t%S\t%M" -a -o "$report/timings.tsv" "$@"
}
{ rustc -vV; cargo -V; wasm-bindgen --version; sccache --version; node --version; chromium --version; uname -a; nproc; git -c safe.directory="$PWD" rev-parse HEAD; } > "$report/environment.txt"
target=${CARGO_TARGET_DIR:-/workspace/target}
export CARGO_TARGET_DIR="$target/shared"
measure native cargo build --locked --release --bin planorama --no-default-features
measure distributions-build sh scripts/build-wasm-distributions.sh
for file in planorama.js planorama_bg.wasm memory-capabilities.json; do
    cmp "output/browser/pkg/$file" "output/wasm-engine/$file"
done
measure browser-memory node tests/browser/memory.cjs output/browser/pkg
measure browser-summary node tests/browser/input-summary.cjs output/browser/pkg
measure browser-tests node tests/browser/wasm.cjs
if [ "$WASM_CI_SUITE" = full ]; then
    measure standalone-lib cargo test --locked --no-default-features --lib
    measure standalone-cli cargo test --locked --no-default-features --test cli help_
fi
measure standalone-memory node tests/browser/memory.cjs
measure standalone-summary node tests/browser/input-summary.cjs
measure standalone-tests node tests/browser/wasm-engine.cjs
sccache --show-stats > "$report/sccache.txt"
du -sk /usr/local/cargo/registry /usr/local/cargo/git "$target" > "$report/cache-sizes.txt"
