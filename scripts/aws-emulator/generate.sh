#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
repo=$PWD
sample=${1:-queue}
case $sample in queue|containment|large) ;; *) echo "Unknown sample: $sample" >&2; exit 2;; esac
tools=$(realpath "${PLANORAMA_EMULATOR_TOOLS:-output/aws-emulator-tools}")
if [ ! -x "$tools/aws-emulator" ] || [ ! -x "$tools/terraform" ]; then
    echo 'Prepare the pinned tools with sh scripts/aws-emulator/prepare.sh' >&2
    exit 1
fi
if [ -z "${PLANORAMA_BIN:-}" ]; then
    cargo build --locked --bin planorama
    PLANORAMA_BIN=${CARGO_TARGET_DIR:-target}/debug/planorama
fi
binary=$(realpath "$PLANORAMA_BIN")
umask 077
mkdir -p output/aws-emulator
scratch=$(mktemp -d)
artifact=$(mktemp -d "$repo/output/aws-emulator/$sample.XXXXXXXX")
cleanup() {
    result=$?
    rm -rf -- "$scratch"
    if [ "$result" -ne 0 ]; then rm -rf -- "$artifact"; fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
mkdir "$scratch/home"
run() {
    "$@" env -i PATH="$PATH" HOME="$scratch/home" sh "$repo/scripts/aws-emulator/capture.sh" "$repo" "$tools" "$binary" "$scratch" "$artifact" "$sample"
}
if [ "$(ls /sys/class/net)" = lo ]; then
    run
else
    run unshare --user --map-root-user --net
fi
printf '%s\n' "$artifact"
