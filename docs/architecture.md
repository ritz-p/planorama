# Architecture and dependency contracts

The processing pipeline is `CLI → TerraformPlan → ArchitectureGraph → ViewGraph → Layout → SVG`, with an optional self-contained HTML viewer around the SVG.
The CLI owns file I/O and coordinates each stage. Named-state inputs preserve the
same boundaries and carry explicit cross-state relationships.

Optional saved-plan [provider evidence](provider-evidence.md) is read and matched
by the CLI using Terraform show. The plan layer validates the embedded snapshot
and dependency lock, then attaches only account IDs to existing provider bindings
before semantic projection. JSON-only ingestion leaves accounts unknown.

Optional [layout diagnostics](layout-diagnostics.md) collect final geometry during
SVG composition, including annotation offsets and cross-state routing, and reuse
runtime layout measurements. The CLI owns the separate report destination.

| Module | Responsibility |
| --- | --- |
| `plan` | Terraform JSON ingestion, native identity, changes, reference/output facts |
| `model` | Shared domain contracts, architecture identity and value-free provenance |
| `semantic` | Architecture transformation and final relationship evidence |
| `provider/aws` | AWS classification, containment, connections, lowering, components, diagnostics and icons |
| `provider/terraform` | Exact built-in Terraform resource icon mapping |
| `icons` | Provider dispatch, shared artwork IDs and presentation measurements |
| `view` | Immutable selection, neighbor expansion, required context and shared presentation measurements |
| `layout` | Containment hierarchy, placement, bounds, ports and routes |
| `svg` | Drawing, text, legends and metadata |
| `html` | Local viewer, explicit value-free resource index from the selected view, and navigation over rendered SVG metadata |

SVG composition places non-spatial annotations after the header, legend and checks,
above the architecture. Internal `MeasuredLegend` and `MeasuredAnnotationSection`
values retain final bounds, child positions and, for the legend, wrapped lines,
action swatches and sample geometry. Rendering serializes these measurements
without wrapping text or packing rows again. Annotation leaf cards retain their
existing measured content fragments; this contract does not change resource
identity, provenance or links. Shared text metrics remain in `view::text`.
The neutral relationship legend has a 960px card-width
cap. Its relationship kinds form a deterministic
one- or two-column grid below the action/color explanation. Multiple columns
require at least 398px per cell with 24px gaps; cells stop widening at 640px.
Narrow single-column cells place the sample above the text. Each grid row reserves
its tallest complete cell, including wrapped text and symbols. The same measured
height controls the canvas, following content and named-state endpoint offsets.
Named states measure one document-level legend from all displayed states at the
shared content width, excluding cross-state gutters. State sections reserve no
local legend height. Legend layout affects section offsets without changing
architecture placement or routing. It remains separate from the numbered Relationship Resources
section.
`svg/annotations.rs` packs rounded cards in deterministic
rows inside ordered sections, reusing the same wrapping and measurement for nested
cards and growing the canvas for oversized cards.
Each section frame ends at its widest occupied row plus padding; the available
canvas width controls child packing independently. Sections remain vertically
stacked, with heights measured from the last occupied row.
Empty areas reserve no height. One enclosing translation shifts all architecture
cards, paths and relationship badges together; annotation cards never enter
`Layout` or routing obstacles. Named-state endpoint coordinates include this shift.
Composition also measures the topmost final card, path and relationship badge.
Geometry that would enter the header or preceding annotation area receives extra
top clearance through a shared downward translation and matching canvas height.
This preserves relative card positions and path shapes, including manually placed
layout fixtures. Named-state heights, endpoint coordinates and routing obstacles
include the same clearance.
Relationship index cards group up to 5 complete numbered entries per card,
continuing to the right without a column cap before wrapping within their own
neutral section. Packing uses the final content width after other annotations can
expand it. Named-state composition prepares each state's cards, resolves the shared
content width, then packs each state's annotations once. Final measured heights
control endpoint offsets, with the cross-state
routing gutter reserved outside that width. Logical
components share a purple parent, with a card per component and a purple heading.
Members use neutral text rows without individual backgrounds, borders or card
wrappers. Each complete wrapped item has one underline in its canonical resource
action color. Row measurements reserve the wrapped text, import badge, underline
and spacing before the next member.

Terraform-backed Logical Resources share a cyan parent above Logical Components;
the relationship index follows both parents. Within each state, `svg/logical.rs`
groups selected logical nodes by canonical provider source, resource type and mode.
The provider key uses `provider.source()`, so explicit and inferred AWS identities
with the same source share a group. Different provider sources or managed/data
modes stay separate. Provider aliases, modules, instances and current/deposed
variants do not add grouping keys; their individual identities and provenance
remain intact. Grouping does not change which resources have logical placement.

Each group has one neutral card with a wrapped resource-type/mode heading and
canonical provider-source line. Nonempty action subsections appear inside the card
in Create, Update, Delete, Replace, Read, Unchanged, Other order, with counts and
canonical action-colored borders. Rows sort by architecture identity within each
subsection; wrappers introduce no resource or DOM identity. Each row retains the full selected
address, action label, deployment-scope label, role, Uses/Dependencies, service icon
and import/move/deposed information, with one action-colored underline below its
complete content. `view::scope` supplies the reusable label from resolved scope
and provider identity. Its wrapped height is reserved before group-card packing.
Data rows use a dashed underline. Groups sort by their keys. Every selected row
remains visible; height grows with action headings, borders, wrapped role
explanations, content, badges, underlines and spacing without a row or height cap.
The provider-neutral `svg::roles` mapping explains Policy and Controller once per
role in the group heading, while individual rows retain their role metadata.

Logical rows retain their original resource DOM IDs and metadata; resource links
target the specific row. Group cards do not introduce architecture identities.
`view::spatial` projects only geometry inputs; the semantic and filtered graphs
keep logical nodes and all selected relationships.
Layout maps spatial results back to graph indices: logical nodes have zero-size
bounds and incident edges have empty paths. They create no routing obstacles.
The SVG summary distinguishes spatial cards, logical resources and logical group
cards while retaining total resource and action counts.
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
underestimate content. Logical-resource rows use the same reference metrics, and
group dimensions include every row's measured content.
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
Terraform built-in icon rules belong in `provider/terraform` and use the provider
identity resolved during ingestion. They add no resource classification,
containment or layout semantics. AWS artwork and Planorama's original built-in
provider icon share the validated build-time SVG parser, symbol reuse and state
namespacing; source notices follow the icon families actually used. See
[icon sources and licenses](icons.md).

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
quality/reordering and CLI reproducibility retain scale coverage. Three expensive
large-fixture unit comparisons run after merge and nightly; focused correctness
and full CLI snapshots remain in PR checks. See [test categories](ci.md#deferred-routing-regressions-253).
Fixtures are
local to each test, with no shared mutable state or test-order dependency.
Run the Docker checks in the [README](../README.md).
