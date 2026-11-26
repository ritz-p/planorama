#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
pilot=${1-reference}
case "$#:$pilot" in
    0:reference|1:reference|1:diff|1:input-file|1:sensitive-path|1:remote-output|1:service-scope|1:saved-provider|1:anonymous-metadata) ;;
    *) echo 'Usage: sh scripts/rust-mutants.sh [reference|diff|input-file|sensitive-path|remote-output|service-scope|saved-provider|anonymous-metadata]' >&2; exit 1 ;;
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
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--test=cli -f src/diff.rs -F "$filter"
        ;;
    remote-output)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -C=--test=cli -f src/semantic/cross_state.rs -F "$filter"
        ;;
    service-scope)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/provider/aws/eventbridge.rs -F "$filter"
        ;;
    saved-provider)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/plan/evidence.rs -F "$filter"
        ;;
    anonymous-metadata)
        expected=1
        output=$output/$pilot
        set -- "$@" -C=--test=cli -f src/view/anonymize.rs -F 'replace Projection::metadata with \(\)$'
        ;;
    sensitive-path)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/plan/changed_paths.rs -F "$filter"
        ;;
    input-file)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--test=cli -f src/cli/files.rs -F "$filter"
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
    sensitive-path) set -- "$@" --exact plan::changed_paths::tests::unavailable_sensitive_and_empty_are_not_conflated plan::operations::tests::sensitive_replacement_paths_are_omitted_before_retaining_keys ;;
    remote-output) set -- "$@" --exact semantic::cross_state::tests::explicit_mapping_resolves_multiple_sources_and_preserves_failures states::cross_state_resolution_never_substitutes_equal_output_values_for_provenance ;;
    service-scope) set -- "$@" --exact provider::aws::resilience_hub::tests::service_reference_requires_the_same_deployment_scope ;;
    saved-provider) set -- "$@" --exact plan::evidence::tests::evidence_keeps_data_current_deposed_and_removed_objects_distinct plan::evidence::tests::verified_static_snapshot_resolves_exact_module_binding_only ;;
    anonymous-metadata) set -- "$@" --exact anonymization::anonymized_exports_remove_identifiers_from_all_exported_bytes anonymization::selection_and_legend_settings_do_not_reintroduce_original_names ;;
    diff) set -- "$@" diff_ ;;
    input-file) set -- "$@" --exact states::hard_linked_output_cannot_truncate_single_or_named_inputs ;;
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
