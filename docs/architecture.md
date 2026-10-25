# Architecture and dependency contracts

The processing pipeline is `JSON → TerraformPlan → ArchitectureGraph → ViewGraph → Layout → SVG`, with an optional self-contained HTML viewer around the SVG.
`src/lib.rs` owns the private modules. The public `generate(&str, &Options)` API
returns an in-memory SVG/HTML and bounded, value-free diagnostic categories.
The CLI owns file I/O, process invocation, output destinations and timings;
`src/main.rs` translates its result into an exit status. Named-state inputs preserve the
same boundaries and carry explicit cross-state relationships.
The CLI and [browser Wasm binding](browser.md) share parsing, semantic projection,
selection and composition through `generation` and `named_inputs`. Named inputs
use the native state partitions and multi-state renderer without filesystem I/O
or implicit cross-state mappings. Both input forms use retained Future state for
resumable jobs, including each state's layout; synchronous callers drive the same
implementation to completion. Browser jobs enforce aggregate budgets across steps
and states and expose fixed pipeline stages through scoped observers. Module inspection
shares `module_inventory` with the native listing command and stops after parsing
and counting original resources. Internal graph and
layout types remain private. Browser timing spans are no-ops; the Worker measures
elapsed generation time. Native-only dependencies and provider evidence are
excluded from `wasm32`; browser generation never invokes Terraform or filesystem I/O.
Explicit [TOML generation settings](configuration.md) are decoded in
`cli::settings`. Typed layers merge common settings, a selected preset and explicit
CLI overrides before populating the existing generation options. File access is
separate from merging; resolved settings use the same validation and pipeline.
`svg::Options` carries address formatting and relationship-legend visibility into
shared measurement and rendering. Legend visibility never changes the graph or
`Layout`; composition uses the measured compact action legend when explanations
are omitted.

The explicit Spatial type allowlist is applied in immutable view projection after
selection/context closure. It refreshes reference-target placement and preserves
reference rows before shared measurement; semantic evidence and selection stay intact.

Optional saved-plan [provider evidence](provider-evidence.md) is read and matched
by the CLI using Terraform show. The plan layer validates the embedded snapshot
and dependency lock, then attaches only account IDs to existing provider bindings
before semantic projection. JSON-only ingestion leaves accounts unknown.

Optional [layout diagnostics](layout-diagnostics.md) collect final geometry during
SVG composition, including annotation offsets and cross-state routing, and reuse
runtime layout measurements. The CLI owns the separate report destination.

The optional [Markdown coverage report](coverage.md) consumes shared accounting
after selection. Its output destination uses the same alias protection as layout
reports; coverage reporting does not participate in SVG composition.
Selection records optional retention reasons during filtering and context closure;
the report reads them from shared coverage accounting, without altering geometry.

Opt-in HTML semantic diagnostics consume selected coverage records through a
bounded serialization bridge. Their viewer adapter reuses resource selection and
the lifecycle reset coordinator; it does not change graph or routing semantics.

HTML neighborhood export projects retained identities onto existing SVG presentation
elements without rerunning layout. The value-free resource index includes containment,
embedded ownership and deployment scope. Mask occluders and bundle junctions carry
resource memberships so removing unrelated content preserves retained path geometry.
The bounded export projection and standalone serialization leave the live viewer intact;
see [export scope and limits](viewer.md).

Opt-in [stage timings](benchmarks.md#cli-stage-timings) use a thread-local collector
for the synchronous generation pipeline. Disabled spans do not read the clock or
allocate report rows. Nested spans subtract child durations for exclusive time;
the collector never changes graph or layout data.

| Module | Responsibility |
| --- | --- |
| `plan` | Terraform JSON ingestion, native identity, changes, reference/output facts |
| `model` | Shared domain contracts, architecture identity and value-free provenance |
| `semantic` | Architecture transformation and final relationship evidence |
| `coverage` | Input/effective resource accounting and structured semantic findings; [contract](coverage.md) |
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
The [relationship legend](relationships.md#legend-and-meaning) is measured once
at the shared content width before state composition. Its height changes state
offsets, not architecture placement or local routing. It remains separate from
the numbered Relationship Resources section.
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
Relationship index packing and number presentation are defined in
[relationship resources](relationships.md#relationship-resource-index);
[component summaries](components.md) define member presentation. Both use shared
text wrapping and measurement. Component member measurements also reserve import
badges and action underlines.

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
canonical action-colored borders. Each action heading has one resource/service
label on its right, padded inside the section frame. Heading, count and label are
measured together; wrapped headers reserve space before the resource rows.
Rows sort by architecture identity within each
subsection; wrappers introduce no resource or DOM identity. Each row retains the full selected
address, deployment-scope label, role, Uses/Dependencies
and import/move/deposed information, with one action-colored underline below its
complete content. Plain actions appear in the counted section headings; individual
rows retain composed import/state-removal operation labels. `view::scope` supplies
the reusable label from resolved scope
and provider identity. Its wrapped height is reserved before group-card packing.
Data rows use a dashed underline. Groups sort by their keys. Every selected row
remains visible; height grows with action headings, borders, wrapped role
explanations, content, badges, underlines and spacing without a row or height cap.
The provider-neutral `svg::roles` mapping explains Identity, Policy and Controller once per
role in the group heading, while individual rows retain their role metadata.

`ResourceRole::Identity` covers principals, identity collections and federation
configuration without changing Placement or proving authentication, permissions,
scope or relationships. The graph and immutable view retain the enum unchanged.
SVG and embedded HTML expose `data-role="identity"` and the same measured heading;
HTML search/navigation/export do not dispatch on a closed role list. The generated
support matrix prints the enum name. There is no serialized role option in the
library/Wasm request API. Consumers of SVG metadata must accept unfamiliar role
strings as descriptive metadata; Rust exhaustive matches must handle the new
variant. Unknown provider/type fallback remains `Unknown`, not Identity.

Logical rows retain their original resource DOM IDs and metadata; resource links
target the specific row. Group cards do not introduce architecture identities.
`view::spatial` projects only geometry inputs; the semantic and filtered graphs
keep logical nodes and all selected relationships.
Layout maps spatial results back to graph indices: logical nodes have zero-size
bounds and incident edges have empty paths. They create no routing obstacles.
The SVG summary distinguishes spatial cards, logical resources and logical group
cards while retaining total resource and action counts.
Value-free reference evidence is available in `resource-references` metadata.
Each entry includes `kind`: `configured`, verified `membership`,
`organization_parent`, `organization_root`, `policy_attachment`, `information`,
`deployment_scope` or `deployment_member`,
independently of presentation ownership and network containment. `information`
has no relationship target. The allowlisted Organizations policy-type display
label is retained separately from this value-free reference metadata. Deployment
correspondence targets may include `deployment_account`, the verified AWS Account
scope identifier. These references do not describe Terraform dependencies.
`organization_root` entries include a zero-based `root_index`; together with the
target Organization identity and enclosing state, it identifies the derived Root.

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
Resource headings display the complete selected address. Shared display metadata
marks labels as resource-specific or service-level. Ordinary/container headers
and labeled embedded members omit the standalone type only for a specific label.
Other headings wrap the full Terraform type and operation without ellipsis.
Unlabeled embedded routes retain their type. Search and details keep the exact
type independently of presentation. Cards share heading text and line-spacing
rules. Ordinary headers
reserve separate columns for top-right resource/service labels and
external indicators; extra heading lines move import badges, addresses, references
and children down before layout and routing. Logical Resources retain their full
group-level type heading without adding redundant per-resource type fields.
Address measurements reserve enough height for both native and qualified headings, including icon
space, and for wrapped move annotations before packing. Uses rows and embedded
Route cards start below that complete header; rendering, child bounds and routing
obstacles use the same offsets. Address formatting never changes resource identity,
metadata or links.
The same module-aware formatting applies to annotation sources, component headings,
parent/Uses references and endpoint titles; [configuration](configuration.md#cli-output-and-addresses)
defines the conservative fallback for historical and unresolved identities.
Imported resources reserve a separate badge row in spatial, logical, embedded
Route and component-member presentations. Shared measurements include this height
when placing subsequent text and child bounds. Badges stay separate from service
icons and external-mode indicators, preserving space for complete addresses.

RDS Cluster Instances, EFS Access Points, KMS Aliases, S3 encryption configurations, Secret Versions and eligible
ACM certificates reuse
the embedded-card measurements and layout projection.
The [resource presentation contract](resource-presentation.md) distinguishes proposed
ownership from effective selected presentation.
Provider rules set `member_owner` from verified explicit references; this is
presentation ownership and adds no semantic Containment edge. Each member keeps
its Terraform identity and contributes once to resource/action counts. Missing
owners after selection release members to ordinary cards. `member-presentations`
metadata records displayed ownership separately from dependency provenance.
`PresentationOwner` includes the target identity, verified attribute and display
labels. `view/presentation.rs` resolves it against the selected state graph, with
independent fallback for missing, ambiguous, nested/cyclic or incompatible owners.
The same effective owner drives cards, bounds, routing ancestry, masks and the HTML
resource context index. Route-specific proof and context closure remain separate.

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

Projection sets `Node.service_name` through provider-aware capabilities,
leaving semantic graphs untouched. The old icon mode is rejected. Card and logical-row measurements
reserve wrapped label heights; SVG and HTML consume the same presentation.

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
Run the Docker checks in the [contributor guide](../CONTRIBUTING.md#検証).
