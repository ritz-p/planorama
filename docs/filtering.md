# Focused diagrams

`--focus ADDRESS` selects an exact native Terraform resource address and its
undirected architecture neighbors, after semantic transformation. The default
depth is 1; `--focus-depth 0` selects only the resource. A lowered association's
address selects its two endpoints. Unknown or ambiguous addresses are errors;
for named states use `--focus STATE:ADDRESS` to disambiguate identical addresses.
An address shared by current and deposed objects is ambiguous.

SNS subscriptions and Lambda event-source mappings retain a logical row, so focusing either at depth zero
selects that row. Its verified endpoint links become unavailable if the endpoints
are excluded. Ordinary neighbor expansion includes its referenced endpoints;
messaging paths require both displayed endpoints.

`--changed-only` retains non-no-op actions and resources with move, import,
removal, drift or other change metadata, including endpoints of changed lowered
relationships and resolved cross-state relationships whose producer output has a
non-no-op action. Combined with focus it intersects these selections first.
Containment ancestors and verified owning Route Tables are then retained automatically. Cross-state relationships
retain both endpoints transitively, even at depth zero, to preserve provenance.
Context nodes can therefore be unchanged or outside the requested depth.

Plan checks/status remain visible. Logical components are shown only when all
their members remain. Empty state panels remain identifiable. Filters create a
new architecture graph without changing parsed plans or the original graph.

Logical Terraform resources participate in the same selection and neighbor rules.
Their references retain stable source identities even if a target is filtered out;
rendering must check target availability before emitting a fragment link.
Logical-resource grouping runs after selection and does not expand it: only
selected rows appear, and groups with no remaining resources are omitted. Links
still identify an individual resource within its group.

These filters also apply to `--format html`. Viewer search operates within the
generated view; it cannot restore resources excluded by CLI selection. See
[the local viewer](viewer.md).

```sh
planorama plan.json --changed-only -o changes.svg
planorama plan.json --focus module.app.aws_instance.web --focus-depth 2
planorama --state app=app.json --state network=network.json --focus app:aws_instance.web
```
