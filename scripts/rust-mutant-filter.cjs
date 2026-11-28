const {readFileSync} = require('node:fs');

const pilots = {
  reference: {
    sources: ['src/plan/address.rs', 'src/provider/cloudflare/proof.rs', 'src/provider/aws/elbv2_tls.rs'],
    filter: 'replace && with \\|\\| in matches_instance$|replace known_configuration_matches -> bool with true$|replace compatible -> bool with true$',
    expected: 3,
    output: 'output/rust-mutants',
    tests: ['plan::', 'provider::cloudflare::', 'provider::aws::elbv2_tls::', 'reference_properties'],
  },
  diff: {
    source: 'src/diff.rs',
    functionName: 'compare',
    changes: [
      ['counts[new] == 1', '==', '!='],
      ['a.provider == b.provider', '==', '!='],
      ['a.mode == b.mode', '==', '!='],
      ['!uncertain.contains(&e.from) && !uncertain.contains(&e.to)', '&&', '||'],
    ],
    expected: 4,
    targets: ['--test=cli'],
    tests: ['diff_'],
  },
  'input-file': {
    source: 'src/cli/files.rs',
    functionName: 'reject_same_file',
    changes: [['input_handle == output_handle', '==', '!=']],
    targets: ['--test=cli'],
    tests: ['--exact', 'states::hard_linked_output_cannot_truncate_single_or_named_inputs'],
  },
  'sensitive-path': {
    source: 'src/plan/changed_paths.rs',
    functionName: 'Collector::walk',
    changes: [['sensitive(&before_sensitive[key]) || sensitive(&after_sensitive[key])', '||', '&&']],
    tests: [
      '--exact',
      'plan::changed_paths::tests::unavailable_sensitive_and_empty_are_not_conflated',
      'plan::operations::tests::sensitive_replacement_paths_are_omitted_before_retaining_keys',
    ],
  },
  'remote-output': {
    source: 'src/semantic/cross_state.rs',
    functionName: 'resolve_step',
    changes: [['!output.complete || output.sources.is_empty()', '||', '&&']],
    targets: ['--lib', '--test=cli'],
    tests: [
      '--exact',
      'semantic::cross_state::tests::explicit_mapping_resolves_multiple_sources_and_preserves_failures',
      'states::cross_state_resolution_never_substitutes_equal_output_values_for_provenance',
    ],
  },
  'service-scope': {
    source: 'src/provider/aws/eventbridge.rs',
    functionName: 'bindings',
    changes: [['role || consumer_region == endpoint_region', '||', '&&']],
    tests: [
      '--exact',
      'provider::aws::resilience_hub::tests::service_reference_requires_the_same_deployment_scope',
    ],
  },
  'saved-provider': {
    source: 'src/plan/evidence.rs',
    functionName: 'Evidence::apply',
    changes: [['node.deposed_key.is_some() || !node.provider.is_aws()', '||', '&&']],
    tests: [
      '--exact',
      'plan::evidence::tests::evidence_keeps_data_current_deposed_and_removed_objects_distinct',
      'plan::evidence::tests::verified_static_snapshot_resolves_exact_module_binding_only',
    ],
  },
  'drift-identity': {
    source: 'src/plan/drift.rs',
    functionName: 'collect',
    changes: [['node.address == address && node.deposed_key == deposed_key', '&&', '||']],
    tests: [
      '--exact',
      'plan::drift::tests::drift_only_and_deposed_objects_are_distinct_and_deterministic',
      'plan::drift::tests::conflicting_drift_is_retained_but_not_attached_to_a_different_resource',
    ],
  },
  'cached-data': {
    source: 'src/plan/collections.rs',
    functionName: 'annotate',
    changes: [['(node.mode == crate::model::EntityMode::Data)', '==', '!=']],
    tests: [
      '--exact',
      'plan::collections::tests::cached_data_ids_are_used_but_explicit_after_null_does_not_fall_back',
      'plan::collections::tests::arn_proof_uses_planned_nested_values_but_never_overrides_explicit_unknown_or_null_changes',
    ],
  },
  'memory-headroom': {
    source: 'src/generation/memory.rs',
    functionName: 'fits',
    changes: [['bytes <= limit', '<=', '>']],
    tests: ['--exact', 'generation::memory::tests::memory_preflight_keeps_headroom_and_never_wraps'],
  },
  'json-budget': {
    source: 'src/generation/budget.rs',
    functionName: "<impl DeserializeSeed<'de> for Seed<'_>>::deserialize",
    changes: [['self.depth >= 64 || self.remaining.get() == 0', '||', '&&']],
    tests: [
      '--exact',
      'generation::budget::tests::structural_limits_are_checked_without_building_a_json_tree',
      'named_inputs::tests::named_jobs_enforce_aggregate_bytes_structure_and_nodes',
    ],
  },
  'named-limits': {
    source: 'src/named_inputs.rs',
    functionName: 'validate',
    changes: [['inputs.len() > 64', '>', '>=']],
    tests: [
      '--exact',
      'named_inputs::tests::named_validation_rejects_names_and_aggregate_size_before_parsing',
      'named_inputs::tests::named_jobs_enforce_aggregate_bytes_structure_and_nodes',
    ],
  },
  'pseudonym-collision': {
    source: 'src/view/anonymize.rs',
    functionName: 'Projection::name',
    changes: [['name == value || self.reserved.contains(&name)', '||', '&&']],
    tests: [
      '--exact',
      'view::anonymize::tests::input_names_in_the_pseudonym_namespace_are_not_reused',
      'view::anonymize::tests::anonymous_projection_keeps_topology_and_resolves_references',
    ],
  },
  'module-instance': {
    source: 'src/model/modules.rs',
    functionName: 'Address::contains',
    changes: [['name == n && (index.is_none() || index == i)', '&&', '||']],
    tests: [
      '--exact',
      'model::modules::tests::structural_module_matching_and_validation',
      'view::filter::module_tests::module_union_intersection_and_identity',
    ],
  },
  'focus-depth': {
    source: 'src/view/filter.rs',
    functionName: 'apply',
    changes: [['!selected.contains(*k)', '!', null]],
    targets: ['--test=cli'],
    tests: [
      '--exact',
      'filtering::focus_preserves_ancestors_without_unrelated_nodes_and_is_deterministic',
      'filtering::changed_only_keeps_lowered_changes_and_focus_depth_is_effective',
    ],
  },
  'changed-context': {
    source: 'src/view/filter.rs',
    functionName: 'apply',
    changes: [['&& placement_available(&all.states[&child.0], child.1, |source|', '&&', '||']],
    targets: ['--test=cli', '--lib'],
    tests: [
      '--exact',
      'filtering::changed_only_keeps_lowered_changes_and_focus_depth_is_effective',
      'semantic::cross_state::tests::changed_only_keeps_output_only_changes_and_respects_focus',
      'view::filter::tests::cache_context_is_retained_only_for_selected_placement_evidence',
    ],
  },
  'coverage-scope': {
    source: 'src/coverage.rs',
    functionName: 'Coverage::collect',
    changes: [['node.provider.is_aws() && node.mode == EntityMode::Managed', '&&', '||']],
    tests: [
      '--exact',
      'coverage::tests::scope_denominators_and_current_deposed_data_identity_are_distinct',
      'coverage::json_report::tests::shared_accounting_selection_and_details_survive_lowering_and_filtering',
    ],
  },
  'check-target': {
    source: 'src/plan/checks.rs',
    functionName: 'resource',
    changes: [['|| address["type"].as_str()? != node.resource_type', '||', '&&']],
    tests: [
      '--exact',
      'plan::checks::tests::conflicting_or_ambiguous_addresses_are_not_associated',
      'plan::checks::tests::checks_keep_statuses_and_exact_resource_associations_without_values',
    ],
  },
  'plan-action': {
    source: 'src/plan.rs',
    functionName: 'parse_action',
    changes: [['actions.contains(&"create") && actions.contains(&"delete")', '&&', '||']],
    tests: [
      '--exact',
      'plan::tests::example_covers_changes_and_module_input_edges',
      'plan::drift::tests::drift_does_not_overwrite_any_planned_action_or_expose_values',
    ],
  },
  'containment-parent': {
    source: 'src/layout/containment.rs',
    functionName: 'parents',
    changes: [['edge.kind == EdgeKind::Containment', '==', '!=']],
    tests: [
      '--exact',
      'layout::containment::tests::hierarchy_preserves_roots_nested_children_ambiguity_and_cycles',
    ],
  },
  'generation-slice': {
    source: 'src/generation.rs',
    functionName: 'Generator::step_with_progress',
    changes: [['units > 4096', '>', '>=']],
    tests: [
      '--exact',
      'named_inputs::tests::resumable_named_preserves_output_selection_and_stage_boundaries',
      'generation::tests::cancellation_and_failure_release_state',
    ],
  },
  'routing-search-budget': {
    source: 'src/work.rs',
    functionName: 'search_work',
    changes: [['remaining - amount', '-', '+']],
    tests: [
      '--exact',
      'generation::tests::search_budget_is_retained_across_steps_and_isolated_between_jobs',
      'generation::tests::named_routing_charges_one_search_budget_across_states',
    ],
  },
  'state-svg-ids': {
    source: 'src/svg/states.rs',
    filter: 'replace prefix -> String with String::new\\(\\)$',
    targets: ['--test=cli'],
    tests: [
      '--exact',
      'states::nested_svg_ids_links_icons_and_markers_are_unique_and_local',
      'states::duplicate_addresses_remain_distinct_with_stable_state_scoping',
    ],
  },
  'frame-contact': {
    source: 'src/layout/routing_shared/borders.rs',
    functionName: 'Index::visit',
    changes: [['straight_low < straight_high', '<', '==']],
    tests: [
      '--exact',
      'layout::routing_shared::borders::tests::perpendicular_boundary_crossings_and_normal_endpoint_stubs_are_free',
      'layout::routing_shared::borders::tests::short_and_long_contacts_measure_empty_stroke_gap_for_every_frame_and_edge',
    ],
  },
  'port-collision': {
    source: 'src/layout/routing_shared/slots.rs',
    functionName: 'opposite_roles_collide',
    changes: [['other != index', '!=', '==']],
    tests: [
      '--exact',
      'layout::routing_shared::slots::tests::coincident_ports_are_scoped_to_nodes_and_distinct_opposite_roles',
      'layout::tests::ports::reciprocal_container_connections_keep_distinct_final_ports',
    ],
  },
  'route-shortcut': {
    source: 'src/layout/routing_shared/scoring.rs',
    functionName: 'Scorer::safe_shortcut',
    changes: [['&& self.safe_border_change(old, new)', '&&', '||']],
    tests: [
      '--exact',
      'layout::routing_shared::shortcuts::tests::shortcuts_preserve_ports_and_reject_obstacles_and_conflicts',
      'layout::routing_shared::shortcuts::tests::route_shortcuts_preserve_configured_frames_and_per_edge_masks',
    ],
  },
  'packing-window': {
    source: 'src/layout/containers/placement.rs',
    functionName: 'select_packing',
    changes: [['compact.0 * 11 && score.1 * 4', '&&', '||']],
    tests: [
      '--exact',
      'layout::containers::placement::selection_tests::packing_selection_matches_lean_cases',
      'layout::containers::placement::selection_tests::packing_uses_relationship_cost_within_the_compact_window',
    ],
  },
  'anonymous-metadata': {
    source: 'src/view/anonymize.rs',
    filter: 'replace Projection::metadata with \\(\\)$',
    targets: ['--test=cli'],
    tests: [
      '--exact',
      'anonymization::anonymized_exports_remove_identifiers_from_all_exported_bytes',
      'anonymization::selection_and_legend_settings_do_not_reintroduce_original_names',
    ],
  },
};

const escape = text => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function filterFor({filter, source, functionName, changes}) {
  if (filter) return filter;
  const lines = readFileSync(source, 'utf8').split('\n');
  const patterns = changes.map(([expression, operator, replacement]) => {
    const line = lines.findIndex(text => text.includes(expression));
    if (line < 0 || lines.slice(line + 1).some(text => text.includes(expression))) {
      throw Error('Expected one matching predicate in ' + source + ': ' + expression);
    }
    const column = lines[line].indexOf(expression) + expression.indexOf(operator) + 1;
    const change = replacement === null ? 'delete ' + operator : 'replace ' + operator + ' with ' + replacement;
    return escape(source + ':' + (line + 1) + ':' + column + ': ' + change + ' in ' + functionName);
  });
  return '^(?:' + patterns.join('|') + ')$';
}

if (require.main === module) console.log(filterFor(pilots[process.argv[2]]));
module.exports = {pilots, filterFor};
