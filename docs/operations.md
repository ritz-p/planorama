# Terraform operation metadata

[Logical architecture components](components.md) retain original resource identities and their complete change information.

Route resources retain their own operation metadata in [embedded child cards](routes.md).
RDS cluster member and EFS access point cards retain the same action, import badge,
move annotation and lifecycle metadata; deposed objects remain separate ordinary cards.

Plan-level checks, including resource preconditions and postconditions, are documented in [Terraform check results](checks.md).

## Resource moves and deposed objects

`previous_address` retains Terraform-reported move provenance without becoming a
reference alias. Spatial cards, logical-resource rows, embedded Route cards, and
each source in the relationship index display `Moved from: ...`. Complete previous
addresses wrap without truncation, with every line included in the measured height.
Component member links retain move context in their accessible titles and point
to the original resource's visible annotation.

Resource elements and lowered edges retain `data-previous-address`, and titles
preserve the complete previous address. Long provenance stays in the index rather than on paths.
The current address and optional deposed key identify separate objects; old objects
retain their own actions and `data-deposed-key`. Current configuration and planned
values apply only to current objects.

## Replacement reasons

Replacement path strings can also be collection keys or set elements. Before retaining paths, both `before_sensitive` and `after_sensitive` are checked. Paths entering a sensitive subtree are omitted in full (including when only a descendant is marked sensitive), so secret keys never reach the model, SVG titles, or data attributes. Other paths remain available; omitted paths are not replaced with misleading truncated paths.

Resource-level `action_reason` is retained verbatim, including unknown future reason strings. `change.replace_paths` stores paths as typed string keys and unsigned integer indices, independently of the planned action. SVG titles and `data-action-reason` / `data-replace-paths` expose these details on resource elements and lowered relationships, with XML escaping. Paths use JSON-array notation so dotted keys, quoted map keys and numeric indices remain unambiguous. They identify attributes without storing their before/after values.

Missing metadata does not alter replacement behavior or ordinary rendering. Explicit empty path lists and empty root paths are preserved. Malformed path entries (non-arrays or steps other than strings/nonnegative integer indices) are ignored as whole paths, never truncated into misleading valid paths. See [replacement-reasons-plan.json](../tests/fixtures/replacement-reasons-plan.json).

## Plan status

`Graph.status` retains the top-level `applyable`, `complete`, and `errored` flags as `Option<bool>`. Missing, null, or non-boolean values remain unknown, distinct from explicit false. Each graph owns the status of its input plan; statuses of different plans are never combined or inferred from resource actions.

When at least one flag is supplied, the SVG header displays all three flags (using `unknown` for missing flags). The `plan-status` group also exposes each known flag as `data-plan-applyable`, `data-plan-complete`, or `data-plan-errored`. Older plans without flags retain their existing output. Incomplete and errored plans remain distinct; these flags do not change resource actions, colors, or the CLI exit status.

`TerraformEntity.metadata`, its card projection `Node.metadata`, and lowered `EdgeChange.metadata` carry operation information separately from `Action`. The existing action palette and legend remain compatible; additional operation labels and SVG data attributes distinguish lifecycle behavior.

## Import and state removal

For `resource_changes[*].change.importing`, Planorama retains the presence of an import object, whether an ID or identity was supplied, and the boolean `unknown` flag. Empty import objects are valid. IDs, identity values, generated configuration, and before/after attributes are deliberately discarded. Import may coexist with a create, update, replacement, or no-op action.

Every retained import object adds a purple `import` badge to the individual spatial
or logical resource, embedded Route, and component member presentation. Empty and
unknown imports receive the same indicator. The synthetic component does not inherit
an import operation, and inline Route entries do not duplicate the table's badge.
The badge has its own measured row, separate from addresses, service icons and the
gray `external` indicator. Spatial action headings omit the redundant `import /`
prefix; embedded type/action headings and annotation operation fields retain the
composed label. Accessible titles preserve the full operation and its order.
Existing `data-import` and the value-free presence/unknown flags remain unchanged.

Ordinary and embedded cards place the operation heading before the address;
it includes the complete type only when no resource-specific label is displayed.
Embedded addresses do not repeat that action. Their address area reserves at least
two lines to keep child cards and routing anchors stable for short identities.
Logical Component members and
Relationship Resources put the complete address before a separate compact
`Action:` field and scope. Relationship entries retain each source's operations.
Logical Resources convey plain actions through their counted section headings;
individual rows retain import/composed state-removal labels, move/deposed identity
and metadata. Synthetic components have no Terraform action. SVG and offline HTML
share these measurements and preserve action colors, counts and resource links.

Exact action arrays `['forget']`, `['create','forget']`, and `['forget','create']` are represented by distinct `StateRemoval` variants. Forgetting removes Terraform's state binding without destroying the old remote object; a combined create/forget operation also creates a new object. These sequences retain the existing `Action::Other` palette/legend category, while resource labels and edge titles explicitly show the operation and order. Ordinary delete remains delete. SVG exposes `data-import`, redacted import flags, and `data-state-removal` on resource elements and lowered relationships.

Only operations explicitly reported in the resource-change list are interpreted. Standalone CLI import/state-rm history, inferred imports based on values, `removed` configuration without a reported forget action, deferred-change lists, action-invocation lists, and unfamiliar action combinations are not interpreted. Unsupported combinations keep the generic action fallback. A non-object `importing` value is ignored.

The supported shapes follow [Terraform's JSON plan implementation](https://github.com/hashicorp/terraform/blob/main/internal/command/jsonplan/plan.go). The regression input is [lifecycle-plan.json](../tests/fixtures/lifecycle-plan.json).

## Resource drift

`relevant_attributes` is retained as value-free provenance in `TerraformPlan.relevant_attributes`. Exact, unique non-deposed resource matches carry relevant paths in operation metadata and SVG titles/data attributes, including lowered edges. Matched drift records expose `relevant`; false means unclassified, not proof that drift was unrelated. Terraform does not supply a destination resource here, so no causal edge is invented. `--diagnostics` lists matched and unresolved sources. Missing metadata preserves existing behavior.

Paths use typed string/index steps. Sensitivity is checked against changes, drift, planned values and prior state; sensitive paths and paths from unresolved sources are withheld. Resource-level reported relevance remains available when paths are redacted. Malformed paths are ignored. No attribute values are retained, compared, or rendered.

`resource_drift` is collected separately in `TerraformPlan.drift`. Each record retains its address, optional deposed key, drift-specific previous address, action category and matching status. Matching uses the exact current address/deposed pair and compatible resource type, mode and provider; previous addresses never become aliases. Conflicting records remain available and produce an unmatched-drift diagnostic rather than changing an unrelated resource.

Matching nodes carry drift action categories in `metadata.drift`, independently of their planned `Action`. Drift-only objects remain visible with no planned change (`Unchanged`); drift deletion is not planned destruction. Consistent duplicate drift records are retained, with deterministic category sets on resources. Contradictory drift-only identities are not arbitrarily assigned a visible resource. Relationship lowering preserves matched drift metadata too.

Default colors and visible action labels still describe the apply plan. SVG exposes `data-drift` categories, and `--diagnostics` reports Terraform-reported drift, including unmatched records. No raw before/after values or identities are stored. This feature consumes reported drift only; it does not query providers or compute drift from state. Unknown action sequences use the existing `Other` category. See [drift-plan.json](../tests/fixtures/drift-plan.json).
