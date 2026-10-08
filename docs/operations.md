# Terraform operation metadata

`Node.metadata` and lowered `EdgeChange.metadata` carry operation information separately from `Action`. The existing action palette and legend remain compatible; additional operation labels and SVG data attributes distinguish lifecycle behavior.

## Import and state removal

For `resource_changes[*].change.importing`, Planorama retains the presence of an import object, whether an ID or identity was supplied, and the boolean `unknown` flag. Empty import objects are valid. IDs, identity values, generated configuration, and before/after attributes are deliberately discarded. Import may coexist with an update, replacement, or no-op action.

Exact action arrays `['forget']`, `['create','forget']`, and `['forget','create']` are represented by distinct `StateRemoval` variants. Forgetting removes Terraform's state binding without destroying the old remote object; a combined create/forget operation also creates a new object. These sequences retain the existing `Action::Other` palette/legend category, while card labels and edge titles explicitly show the operation and order. Ordinary delete remains delete. SVG exposes `data-import`, redacted import flags, and `data-state-removal` on cards and lowered relationships.

Only operations explicitly reported in the resource-change list are interpreted. Standalone CLI import/state-rm history, inferred imports based on values, `removed` configuration without a reported forget action, deferred-change lists, action-invocation lists, and unfamiliar action combinations are not interpreted. Unsupported combinations keep the generic action fallback. A non-object `importing` value is ignored.

The supported shapes follow [Terraform's JSON plan implementation](https://github.com/hashicorp/terraform/blob/main/internal/command/jsonplan/plan.go). The regression input is [lifecycle-plan.json](../tests/fixtures/lifecycle-plan.json).
