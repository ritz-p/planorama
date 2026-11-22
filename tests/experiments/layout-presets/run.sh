#!/bin/sh
set -eu
root=$(git rev-parse --show-toplevel)
cd "$root"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
git archive -o "$scratch/source.tar" HEAD
mkdir "$scratch/source"
tar -xf "$scratch/source.tar" -C "$scratch/source"
file=src/layout/containers/placement.rs
cp "$scratch/source/$file" "$scratch/original.rs"
for policy in standard compact wide readable; do
    case "$policy" in
        standard) cp "$scratch/original.rs" "$scratch/source/$file" ;;
        compact) sed 's/(width as u128 \* 10).max(height as u128 \* 17)/width as u128 * height as u128/' "$scratch/original.rs" > "$scratch/source/$file" ;;
        wide) sed 's/height as u128 \* 17/height as u128 * 30/' "$scratch/original.rs" > "$scratch/source/$file" ;;
        readable) sed 's/compact.0 \* 11/compact.0 * 15/;s/compact.1 \* 5/compact.1 * 7/' "$scratch/original.rs" > "$scratch/source/$file" ;;
    esac
    if [ "$policy" != standard ] && cmp -s "$scratch/original.rs" "$scratch/source/$file"; then
        echo "Packing policy changed. Update the $policy experiment." >&2
        exit 1
    fi
    (cd "$scratch/source" && CARGO_TARGET_DIR="$scratch/target" cargo build --locked --bin planorama)
    cp "$scratch/target/debug/planorama" "$scratch/$policy"
done
node tests/experiments/layout-presets/compare.cjs "$scratch/standard" "$scratch/compact" "$scratch/wide" "$scratch/readable"
