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
| `view` | Immutable selection, neighbor expansion, required context and shared presentation measurements |
| `layout` | Containment hierarchy, placement, bounds, ports and routes |
| `svg` | Drawing, text, legends and metadata |

SVG composition places non-spatial annotations after the header, legend and checks,
above the architecture. The neutral relationship legend measures wrapped action
samples and explanatory rows at the diagram's available width. The same measured
height controls the canvas, following content and named-state endpoint offsets.
It remains separate from the numbered Relationship Resources section.
`svg/annotations.rs` packs rounded cards in deterministic
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

Consumer cards show every Uses/Dependencies row from the same evidence, with
provider labels supplied during projection. Shared label and wrapping metrics
determine intrinsic width and height before flat/nested packing, scope placement
and routing. Reference-driven widths grow from 320 to 640 pixels; longer labels
wrap at 11px monospace with 18px line spacing and no row limit. Measurements
reserve the Unavailable prefix so filtering and spatial projection cannot
underestimate content. Logical annotation cards use the same reference metrics.
Full Terraform addresses include module/instance keys, remote state and deposed
identity. UI links never add routing edges.

Address wrapping prefers separators between address elements. Dots inside quoted
instance keys stay part of the key, including after an escaped quote or a forced
line break. An individual token wider than the available space falls back to
UTF-8 character boundaries; every delimiter and space is preserved. Ordinary
explanatory text keeps its separate wrapping behavior.
Resource headings display the complete selected address. Shared measurements
reserve enough height for both native and qualified headings, including icon
space, and for wrapped move annotations before packing. Uses rows and embedded
Route cards start below that complete header; rendering, child bounds and routing
obstacles use the same offsets. Address formatting never changes resource identity,
metadata or links.
Imported resources reserve a separate badge row in spatial, logical, embedded
Route and component-member presentations. Shared measurements include this height
when placing subsequent text and child bounds. Badges stay separate from service
icons and external-mode indicators, preserving space for complete addresses.

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
