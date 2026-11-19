#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mode=${1:-pr}
case $mode in pr|full) ;; *) echo 'Expected pr or full' >&2; exit 2;; esac
: "${PLANORAMA_BIN:?Build Planorama and set PLANORAMA_BIN before checking samples}"
node --test tests/emulator/guards.test.cjs
if [ "$mode" = pr ]; then sh tests/emulator/isolation.sh; fi
log=$(mktemp)
trap 'rm -f -- "$log"' EXIT
capture() {
    start=$(date +%s)
    if ! sh scripts/aws-emulator/generate.sh "$@" >"$log"; then
        echo "Capture failed: $*" >&2
        exit 1
    fi
    captured=$(tail -n 1 "$log")
    test -d "$captured"
    printf '%s took %ss\n' "$*" "$(($(date +%s) - start))"
}
if [ "$mode" = pr ]; then
    capture containment --create-only
else
    capture queue
    capture containment
fi
node scripts/aws-emulator/create-snapshot.cjs "$captured/create.json" >"$log"
cmp "$log" examples/aws-emulator/containment/create.json
cmp "$captured/create.svg" examples/containment.svg
if [ "$mode" = full ]; then
    capture large
    cmp "$captured/stress.json" examples/terraform-large/plan.json
fi
