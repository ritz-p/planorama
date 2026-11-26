#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
pilot=${1-reference}
case "$#:$pilot" in
    0:reference|1:reference|1:diff|1:input-file|1:sensitive-path|1:remote-output|1:service-scope|1:saved-provider|1:drift-identity|1:cached-data|1:memory-headroom|1:json-budget|1:named-limits|1:pseudonym-collision|1:module-instance|1:focus-depth|1:changed-context|1:anonymous-metadata) ;;
    *) echo 'Usage: sh scripts/rust-mutants.sh [reference|diff|input-file|sensitive-path|remote-output|service-scope|saved-provider|drift-identity|cached-data|memory-headroom|json-budget|named-limits|pseudonym-collision|module-instance|focus-depth|changed-context|anonymous-metadata]' >&2; exit 1 ;;
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
    drift-identity)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/plan/drift.rs -F "$filter"
        ;;
    cached-data)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/plan/collections.rs -F "$filter"
        ;;
    memory-headroom)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/generation/memory.rs -F "$filter"
        ;;
    json-budget)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/generation/budget.rs -F "$filter"
        ;;
    named-limits)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/named_inputs.rs -F "$filter"
        ;;
    pseudonym-collision)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/view/anonymize.rs -F "$filter"
        ;;
    module-instance)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--lib -f src/model/modules.rs -F "$filter"
        ;;
    focus-depth)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--test=cli -f src/view/filter.rs -F "$filter"
        ;;
    changed-context)
        expected=1
        output=$output/$pilot
        filter=$(node scripts/rust-mutant-filter.cjs "$pilot")
        set -- "$@" -C=--test=cli -C=--lib -f src/view/filter.rs -F "$filter"
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
    drift-identity) set -- "$@" --exact plan::drift::tests::drift_only_and_deposed_objects_are_distinct_and_deterministic plan::drift::tests::conflicting_drift_is_retained_but_not_attached_to_a_different_resource ;;
    cached-data) set -- "$@" --exact plan::collections::tests::cached_data_ids_are_used_but_explicit_after_null_does_not_fall_back plan::collections::tests::arn_proof_uses_planned_nested_values_but_never_overrides_explicit_unknown_or_null_changes ;;
    memory-headroom) set -- "$@" --exact generation::memory::tests::memory_preflight_keeps_headroom_and_never_wraps ;;
    json-budget) set -- "$@" --exact generation::budget::tests::structural_limits_are_checked_without_building_a_json_tree named_inputs::tests::named_jobs_enforce_aggregate_bytes_structure_and_nodes ;;
    named-limits) set -- "$@" --exact named_inputs::tests::named_validation_rejects_names_and_aggregate_size_before_parsing named_inputs::tests::named_jobs_enforce_aggregate_bytes_structure_and_nodes ;;
    pseudonym-collision) set -- "$@" --exact view::anonymize::tests::input_names_in_the_pseudonym_namespace_are_not_reused view::anonymize::tests::anonymous_projection_keeps_topology_and_resolves_references ;;
    module-instance) set -- "$@" --exact model::modules::tests::structural_module_matching_and_validation view::filter::module_tests::module_union_intersection_and_identity ;;
    focus-depth) set -- "$@" --exact filtering::focus_preserves_ancestors_without_unrelated_nodes_and_is_deterministic filtering::changed_only_keeps_lowered_changes_and_focus_depth_is_effective ;;
    changed-context) set -- "$@" --exact filtering::changed_only_keeps_lowered_changes_and_focus_depth_is_effective semantic::cross_state::tests::changed_only_keeps_output_only_changes_and_respects_focus view::filter::tests::cache_context_is_retained_only_for_selected_placement_evidence ;;
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
