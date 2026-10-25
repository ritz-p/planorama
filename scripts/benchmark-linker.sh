#!/bin/sh
set -eu
case "${1:-all}" in
    all) linkers='system mold' ;;
    system|mold) linkers=$1 ;;
    *) echo 'Usage: benchmark-linker.sh [all|system|mold]' >&2; exit 2 ;;
esac
test "$(rustc -vV | sed -n 's/^host: //p')" = x86_64-unknown-linux-gnu
root=$(pwd)
report="$root/output/linker-benchmark"
mkdir -p "$report"
{ rustc -vV; cc --version; ld --version; mold --version; uname -a; nproc; } > "$report/environment.txt"
source_dir=$(mktemp -d /tmp/planorama-linker-source.XXXXXX)
git ls-files -z | tar --null -T - -cf - | tar -xf - -C "$source_dir"
cp scripts/native-linker.sh "$source_dir/native-linker.sh"
chmod +x "$source_dir/native-linker.sh"
cd "$source_dir"
export CARGO_TARGET_DIR=/tmp/planorama-linker-benchmark-target
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$source_dir/native-linker.sh"
export CARGO_INCREMENTAL=0
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
for linker in $linkers; do
    export PLANORAMA_LINKER=$linker
    cargo clean --target-dir "$CARGO_TARGET_DIR"
    for profile in test release; do
        for temperature in cold warm; do
            name="$linker-$profile-$temperature"
            export PLANORAMA_LINK_TIMINGS="$report/$name-links"
            rm -rf "$PLANORAMA_LINK_TIMINGS"
            mkdir -p "$PLANORAMA_LINK_TIMINGS"
            if [ "$profile" = test ]; then
                set -- test --locked --all-targets --all-features --no-run
            else
                set -- build --locked --release --bin planorama
            fi
            /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$name.tsv" cargo "$@" --timings > "$report/$name.log" 2>&1
            cp "$CARGO_TARGET_DIR/cargo-timings/cargo-timing.html" "$report/$name.html"
        done
    done
    readelf -p .comment "$CARGO_TARGET_DIR/release/planorama" > "$report/$linker-linker.txt"
    if [ "$linker" = mold ]; then
        grep -F 'mold 2.40.4' "$report/$linker-linker.txt"
    fi
    /usr/bin/time -f '%e\t%U\t%S\t%M' -o "$report/$linker-test-run.tsv" cargo test --locked --all-targets --all-features > "$report/$linker-test-run.log" 2>&1
    "$CARGO_TARGET_DIR/release/planorama" examples/plan.json -o "$report/$linker.svg"
done
if [ -f "$report/system.svg" ] && [ -f "$report/mold.svg" ]; then
    cmp "$report/system.svg" "$report/mold.svg"
fi
cargo clean --target-dir "$CARGO_TARGET_DIR"
rm -rf "$source_dir"
