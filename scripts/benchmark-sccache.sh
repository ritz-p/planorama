#!/bin/sh
set -eu
root=$(pwd)
report="$root/output/sccache-benchmark"
mkdir -p "$report"
source_dir=$(mktemp -d /tmp/planorama-sccache-source.XXXXXX)
git ls-files -z | tar --null -T - -cf - | tar -xf - -C "$source_dir"
cp scripts/native-linker.sh "$source_dir/native-linker.sh"
chmod +x "$source_dir/native-linker.sh"
cd "$source_dir"
export CARGO_TARGET_DIR="$source_dir/target"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$source_dir/native-linker.sh"
SCCACHE_DIR=$(mktemp -d /tmp/planorama-sccache-cache.XXXXXX)
export SCCACHE_DIR
export SCCACHE_CACHE_SIZE=2G SCCACHE_SERVER_PORT=4237 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER
{ rustc -vV; sccache --version; uname -a; nproc; } > "$report/environment.txt"
run_case() {
    name=$1
    shift
    if [ -n "${RUSTC_WRAPPER:-}" ]; then
        SCCACHE_START_SERVER=1 SCCACHE_NO_DAEMON=1 /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$name-server.tsv" sccache > "$report/$name-server.log" 2>&1 &
        server=$!
        attempts=0
        until sccache --show-stats >/dev/null 2>&1; do
            attempts=$((attempts + 1))
            test "$attempts" -lt 20
            sleep 0.1
        done
        sccache --zero-stats > /dev/null
    fi
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$name.tsv" cargo "$@" > "$report/$name.log" 2>&1
    if [ -n "${RUSTC_WRAPPER:-}" ]; then
        sccache --show-stats > "$report/$name-stats.txt"
        sccache --show-stats --stats-format json > "$report/$name-stats.json"
        sccache --stop-server >> "$report/$name-server.log"
        wait "$server"
    fi
}
export RUSTC_WRAPPER=
run_case native-baseline test --locked --all-targets --all-features --no-run
cargo clean
export RUSTC_WRAPPER=sccache
run_case native-cold test --locked --all-targets --all-features --no-run
cargo clean
run_case native-warm test --locked --all-targets --all-features --no-run
run_case native-target-hit test --locked --all-targets --all-features --no-run
printf '\n' >> src/lib.rs
run_case native-partial test --locked --all-targets --all-features --no-run
sed -i 's/^serde_json = "1"$/serde_json = { version = "1", features = ["raw_value"] }/' Cargo.toml
run_case native-dependency-feature test --locked --all-targets --all-features --no-run
sed -i 's/^serde_json = { version = "1", features = \["raw_value"\] }$/serde_json = "1"/' Cargo.toml
export CARGO_INCREMENTAL=1 RUSTC_WRAPPER=
run_case native-incremental test --locked --all-targets --all-features --no-run
printf '\n' >> src/lib.rs
run_case native-incremental-partial test --locked --all-targets --all-features --no-run
export CARGO_INCREMENTAL=0 RUSTC_WRAPPER=
run_case wasm-baseline build --locked --release --no-default-features --lib --target wasm32-unknown-unknown
cp "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/planorama.wasm" "$report/baseline.wasm"
cargo clean
export RUSTC_WRAPPER=sccache
run_case wasm-cold build --locked --release --no-default-features --lib --target wasm32-unknown-unknown
cmp "$report/baseline.wasm" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/planorama.wasm"
cargo clean
run_case wasm-warm build --locked --release --no-default-features --lib --target wasm32-unknown-unknown
cmp "$report/baseline.wasm" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/planorama.wasm"
du -sk "$SCCACHE_DIR" > "$report/cache-size.txt"
/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/cache-pack.tsv" tar -czf "$report/cache.tar.gz" -C "$SCCACHE_DIR" .
mkdir "$source_dir/restored-cache"
/usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/cache-unpack.tsv" tar -xzf "$report/cache.tar.gz" -C "$source_dir/restored-cache"
wc -c "$report/cache.tar.gz" > "$report/archive-size.txt"
cargo clean
rm -rf "$source_dir"
rm -rf "$SCCACHE_DIR"
