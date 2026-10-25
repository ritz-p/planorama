#!/bin/sh
set -eu
root=$(pwd)
report="$root/output/binding-cache-benchmark"
mkdir -p "$report"
directory=$(mktemp -d /tmp/planorama-binding-cache.XXXXXX)
export CARGO_TARGET_DIR="$directory/target" SCCACHE_DIR="$directory/cache"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc
export CARGO_INCREMENTAL=0 SCCACHE_CACHE_SIZE=2G SCCACHE_SERVER_PORT=4238
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER
for mode in baseline cold warm; do
    cargo clean --target-dir "$CARGO_TARGET_DIR"
    export RUSTC_WRAPPER=
    if [ "$mode" != baseline ]; then
        export RUSTC_WRAPPER=sccache
        SCCACHE_START_SERVER=1 SCCACHE_NO_DAEMON=1 /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$mode-server.tsv" sccache > "$report/$mode-server.log" 2>&1 &
        server=$!
        attempts=0
        until sccache --show-stats >/dev/null 2>&1; do
            attempts=$((attempts + 1))
            test "$attempts" -lt 20
            sleep 0.1
        done
        sccache --zero-stats > /dev/null
    fi
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$mode.tsv" cargo +1.85.1 install wasm-bindgen-cli --version 0.2.100 --locked --force --root "$directory/install" > "$report/$mode.log" 2>&1
    "$directory/install/bin/wasm-bindgen" --version > "$report/$mode-version.txt"
    sha256sum "$directory/install/bin/wasm-bindgen" > "$report/$mode-binary.txt"
    if [ "$mode" != baseline ]; then
        sccache --show-stats > "$report/$mode-stats.txt"
        sccache --show-stats --stats-format json > "$report/$mode-stats.json"
        sccache --stop-server >> "$report/$mode-server.log"
        wait "$server"
    fi
done
/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/tool-hit.tsv" "$directory/install/bin/wasm-bindgen" --version
du -sk "$SCCACHE_DIR" > "$report/cache-size.txt"
/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/cache-pack.tsv" tar -czf "$report/cache.tar.gz" -C "$SCCACHE_DIR" .
mkdir "$directory/restored-cache"
/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/cache-unpack.tsv" tar -xzf "$report/cache.tar.gz" -C "$directory/restored-cache"
wc -c "$report/cache.tar.gz" > "$report/archive-size.txt"
"$directory/install/bin/wasm-bindgen" --target no-modules --no-typescript --out-dir "$report/generated" "$root/output/sccache-benchmark/baseline.wasm"
wasm-bindgen --target no-modules --no-typescript --out-dir "$report/reference" "$root/output/sccache-benchmark/baseline.wasm"
cmp "$report/generated/baseline.js" "$report/reference/baseline.js"
cmp "$report/generated/baseline_bg.wasm" "$report/reference/baseline_bg.wasm"
rm -rf "$directory"
