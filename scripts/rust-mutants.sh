#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$(cargo mutants --version)" != 'cargo-mutants 25.3.1' ]; then
    echo 'Install cargo-mutants 25.3.1 before running the pilot.' >&2
    exit 1
fi
export CARGO_PROFILE_TEST_OPT_LEVEL=0 CARGO_PROFILE_TEST_DEBUG=0
set -- -j 1 --no-shuffle --build-timeout 420 --timeout 90 --all-features \
    -C=--locked -C=--lib \
    -f src/plan/address.rs -f src/provider/cloudflare/proof.rs -f src/provider/aws/elbv2_tls.rs \
    -F 'replace && with \|\| in matches_instance$|replace known_configuration_matches -> bool with true$|replace compatible -> bool with true$'
selection=$(cargo mutants "$@" --list)
if [ "$(printf '%s\n' "$selection" | wc -l)" -ne 3 ]; then
    printf 'Expected the three pilot mutations, found:\n%s\n' "$selection" >&2
    exit 1
fi
mkdir -p output/rust-mutants
exec timeout -k 10s 20m cargo mutants "$@" -o output/rust-mutants -- -- \
    plan:: provider::cloudflare:: provider::aws::elbv2_tls:: reference_properties
