# Focused diagrams

`--focus ADDRESS` selects an exact native Terraform resource address and its
undirected architecture neighbors, after semantic transformation. The default
depth is 1; `--focus-depth 0` selects only the resource. A lowered association's
address selects its two endpoints. Unknown or ambiguous addresses are errors;
for named states use `--focus STATE:ADDRESS` to disambiguate identical addresses.
An address shared by current and deposed objects is ambiguous.

`--changed-only` retains non-no-op actions and resources with move, import,
removal, drift or other change metadata, including endpoints of changed lowered
relationships and resolved cross-state relationships whose producer output has a
non-no-op action. Combined with focus it intersects these selections first.
Containment ancestors are then retained automatically. Cross-state relationships
retain both endpoints transitively, even at depth zero, to preserve provenance.
Context nodes can therefore be unchanged or outside the requested depth.

Plan checks/status remain visible. Logical components are shown only when all
their members remain. Empty state panels remain identifiable. Filters create a
new architecture graph without changing parsed plans or the original graph.

```sh
planorama plan.json --changed-only -o changes.svg
planorama plan.json --focus module.app.aws_instance.web --focus-depth 2
planorama --state app=app.json --state network=network.json --focus app:aws_instance.web
```
