#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
pilot=${1-reference}
case "$#:$pilot" in
    0:reference|1:reference|1:diff) ;;
    *) echo 'Usage: sh scripts/rust-mutants.sh [reference|diff]' >&2; exit 1 ;;
esac
if [ "$(cargo mutants --version)" != 'cargo-mutants 25.3.1' ]; then
    echo 'Install cargo-mutants 25.3.1 before running the pilot.' >&2
    exit 1
fi

export CARGO_PROFILE_TEST_OPT_LEVEL=0 CARGO_PROFILE_TEST_DEBUG=0
set -- -j 1 --no-shuffle --build-timeout 420 --timeout 90 --all-features -C=--locked
output=output/rust-mutants
case "$pilot" in
    reference)
        expected=3
        set -- "$@" -C=--lib \
            -f src/plan/address.rs -f src/provider/cloudflare/proof.rs -f src/provider/aws/elbv2_tls.rs \
            -F 'replace && with \|\| in matches_instance$|replace known_configuration_matches -> bool with true$|replace compatible -> bool with true$'
        ;;
    diff)
        expected=4
        output=$output/diff
        filter=$(node scripts/diff-mutant-filter.cjs)
        set -- "$@" -C=--test=cli -f src/diff.rs -F "$filter"
        ;;
esac

selection=$(cargo mutants "$@" --list)
count=$(printf '%s\n' "$selection" | grep -c . || true)
if [ "$count" -ne "$expected" ]; then
    printf 'Expected %s %s mutations, found %s:\n%s\n' "$expected" "$pilot" "$count" "$selection" >&2
    exit 1
fi
printf '%s\n' "$selection"
mkdir -p "$output"
set -- "$@" -o "$output" -- --
case "$pilot" in
    reference) set -- "$@" plan:: provider::cloudflare:: provider::aws::elbv2_tls:: reference_properties ;;
    diff) set -- "$@" diff_ ;;
esac
status=0
timeout -k 10s 20m cargo mutants "$@" || status=$?
case "$status" in
    0|2)
        if ! grep -E '^test result: ok\. [1-9][0-9]* passed;' "$output/mutants.out/log/baseline.log"; then
            echo "The $pilot baseline must execute at least one passing test." >&2
            exit 1
        fi
        ;;
esac
exit "$status"
