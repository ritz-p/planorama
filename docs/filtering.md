# Focused diagrams

`--modules ADDRESS...` selects the union of Terraform module subtrees. Repeat the
option or pass several space-separated addresses; commas are part of keys, not
separators. Put the input before this variable-length option. Selection is based
on original Terraform identities, including logical rows, embedded Routes and
lowered relationships, rather than the module of a display group.

`module.app` includes its direct resources and descendants, across all count and
for_each instances, but never `module.application`. An index constrains only its
own path segment: `module.app.module.db[2]` selects db instance 2 under every app
instance; `module.app[0].module.db` selects all db instances under app instance 0.
String keys retain dots, commas, colons, quotes and escapes. Resource addresses,
malformed syntax and unknown modules are errors. Known empty/configuration-only
modules are valid empty selections; configuration does not prove any indexed
instance exists. Validation precedes other filters.

For named inputs use `STATE:MODULE_ADDRESS`. An unqualified selector matching
multiple states is an error. Overlapping or repeated selectors do not duplicate
resources, and argument order does not change the diagram. Root resources have
no module selector; omit `--modules` to include them normally.

Modules, focus and changed-only intersect before context closure. No general
dependency expansion is added: focus depth affects only focus. Containment parents,
verified Route Table owners and cross-state provenance can remain outside the
selected modules. A selected lowered relationship uses its existing endpoint
representation. Excluded logical targets remain unavailable without fragment
links. Module selection creates no new module containers or interactive picker.

```sh
planorama plan.json --modules module.app module.database -o selected.svg
planorama plan.json --modules 'module.app["blue,west:1"]' --modules module.database --format html -o selected.html
planorama --state app=app.json --state network=network.json --modules app:module.api network:module.vpc -o selected.svg
```

The examples use POSIX shell quoting. Quote the entire selector and preserve its
literal double quotes using your shell's native argument rules; Windows PowerShell
versions can differ when forwarding quotes to native commands.

`--focus ADDRESS` selects an exact native Terraform resource address and its
undirected architecture neighbors, after semantic transformation. The default
depth is 1; `--focus-depth 0` selects only the resource. A lowered association's
address selects its two endpoints. Unknown or ambiguous addresses are errors;
for named states use `--focus STATE:ADDRESS` to disambiguate identical addresses.
An address shared by current and deposed objects is ambiguous.

SNS subscriptions, Lambda event-source mappings and EventBridge targets retain a logical row, so focusing one at depth zero
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

RDS cluster membership does not expand selection. If the cluster is excluded,
selected instances return to ordinary visible cards. If an instance is excluded,
only the remaining selected members occupy the cluster card.

If an ElastiCache Replication Group's referenced Subnet Group is excluded, its
inferred VPC placement is removed and the replication group remains visible.
That removed placement does not retain an otherwise unselected VPC as context.

Plan checks/status remain visible. Logical components are shown only when all
their members remain. Empty state panels remain identifiable. Filters create a
new architecture graph without changing parsed plans or the original graph.

The optional `--coverage-report PATH` Markdown report preserves input counts and reports selected/excluded instances.
Excluded includes metadata suppressed during projection and resources outside CLI
selection; it does not mean those resources are absent from Terraform. Required
context is included in selected counts. See [coverage accounting](coverage.md).

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
