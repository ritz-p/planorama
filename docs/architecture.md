# Architecture and dependency contracts

The processing pipeline is `CLI → TerraformPlan → ArchitectureGraph → ViewGraph → Layout → SVG`.
The CLI owns file I/O and coordinates each stage. Named-state inputs preserve the
same boundaries and carry explicit cross-state relationships.

| Module | Responsibility |
| --- | --- |
| `plan` | Terraform JSON ingestion, native identity, changes, reference/output facts |
| `model` | Shared domain contracts, architecture identity and value-free provenance |
| `semantic` | Architecture transformation and final relationship evidence |
| `provider/aws` | AWS classification, containment, connections, lowering, components, diagnostics and icons |
| `view` | Immutable selection, neighbor expansion and required context |
| `layout` | Containment hierarchy, placement, bounds, ports and routes |
| `svg` | Drawing, text, legends and metadata |

SVG composition places non-spatial annotations after the header, legend and checks,
above the architecture. `svg/annotations.rs` packs rounded cards in deterministic
rows inside ordered sections, reusing the same wrapping and measurement for nested
cards and growing the canvas for oversized cards.
Empty areas reserve no height. One enclosing translation shifts all architecture
cards, paths and relationship badges together; annotation cards never enter
`Layout` or routing obstacles. Named-state endpoint coordinates include this shift.
Relationship index cards group up to 10 complete numbered entries per card,
continuing to the right before wrapping within their own neutral section. Logical
components share a purple parent, with a card per component and action-colored
member reference cards inside it.

Terraform-backed Logical Resources share a cyan parent above Logical Components;
the relationship index follows both parents. Logical cards use their original
resource DOM IDs and retain mode, action,
move/import/deposed information. `view::spatial` projects only geometry inputs;
the semantic and filtered graphs keep logical nodes and all selected relationships.
Layout maps spatial results back to graph indices: logical nodes have zero-size
bounds and incident edges have empty paths. They create no routing obstacles.
The SVG summary distinguishes spatial and logical cards while retaining totals.
Value-free reference evidence is available in `resource-references` metadata.

Verified Route ownership uses the same projection boundary: the Route node remains
in the graph, while its card is embedded in the owning table. `view/routes.rs`
measures the shared rows before spatial placement; the expanded table reserves
the complete child height. Final routing uses the child's bounds and ports for its
next-hop Association. Layout ancestry supports exiting the parent without adding
a semantic Containment edge. See [routes](routes.md).

Consumer cards show compact Uses rows from the same evidence, with provider labels
supplied during projection. Shared row selection determines both content height
and rendering. Logical cards also show their own references; UI links never add
routing edges. Full lists remain in accessible titles when the four-row limit
is exceeded.

Ingestion does not import provider rules, layout or SVG. Layout, view and SVG use
the `model::architecture` facade, which excludes Terraform ingestion contracts.
AWS resource-type and attribute rules belong in `provider/aws`; rendering and
layout do not infer semantic relationships.

The semantic input projects Terraform facts into classified cards without mutating
the plan. AWS transformation applies containment, SG connections, association
lowering, data visibility and logical components. Native relationships are finalized
after these transformations. See [models](models.md) and [AWS rules](aws.md).

`ArchitectureRelationship` uses stable architecture IDs and resource/reference
provenance. `Graph.edges` is the indexed adapter used by geometry. Layout uses the
model's derived-containment predicate to reserve annotation space.

View projection retains relationships only when their required endpoints survive.
`ViewGraph` and `ViewArchitecture` have private constructors and no mutable deref.
Even unfiltered CLI output passes through projection.

Implementation submodules remain private; entry points are crate-visible.
Unit tests can access private code through child modules loaded from `tests/unit`.
Reference-resolution matrices in `tests/unit/semantic/resolution.rs` call the parser,
semantic transformation and diagnostic collector directly. Integration tests invoke
the CLI for I/O, options, diagnostics, redaction, filtering and named-state contracts.
Renderer feature tests reuse their SVG for snapshot assertions; dedicated tests
cover repeated rendering and input reordering. Generic marker/stub checks use
focused fixtures. Large-fixture geometry checks reuse their layout, while per-edge
quality/reordering and CLI reproducibility retain scale coverage. Fixtures are
local to each test, with no shared mutable state or test-order dependency.
Run the Docker checks in the [README](../README.md).
