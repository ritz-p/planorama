# Layout and routing

| Stage | Contract | Implementation |
| --- | --- | --- |
| Hierarchy | Unique parents, deterministic roots/children, ancestry, cycle handling | `layout/containment.rs` |
| Geometry | Bounds, points, four-sided ports | `layout/geometry.rs` |
| Shared routing | Obstacles, coverage, scoring, orthogonal search | `layout/routing_shared` |
| Pipeline | Hierarchy → placement → routing → final geometry | `layout/pipeline.rs` |

`Layout::new` builds one containment tree. Ambiguous or cyclic containment remains
an edge rather than nesting. Both placement strategies share the tree and actual
card/container bounds.

Logical resources and their group cards do not enter this pipeline. A spatial view
projection removes only their geometry participation, retaining graph-index alignment with
zero bounds and empty incident paths in the final layout. Their Terraform nodes,
relationships and evidence remain in the semantic/view graph and SVG metadata.
The projection also normalizes node identity order before placement, mapping
results back to the caller's indices so reordered flat inputs keep the same ports.

Embedded Route cards share their owning table's placement footprint. The table
reserves the measured child content before placement, and next-hop cards reserve
capacity for the additional ports. Final routing uses each child's bounds and a
layout-only parent relation, protects the table header and sibling entries, and
draws separate directed Associations to visible next hops, reserving numbered
badges at both child and target ports. The background is drawn
before the paths so the segment inside the table remains visible. Cross-state Route endpoints use the table's
border while retaining the child identity and explicit attachment metadata.
When embedded Routes are restored, rerouting initializes existing edges from their
previously selected sides and assigns slots on those sides. This preserves the
port capacity used during height refinement; resetting every port to the right
can overlap numbered reservations on compact cards. Newly visible edges start
on the right and participate in the same numbered-port selection.

RDS cluster members share this measured footprint and layout-only parent relation.
Member content expands the cluster before routing, including wrapped addresses,
scope and dependency links. References involving members remain navigable text
and semantic metadata; other cluster relationships use its expanded bounds.

## Placement

Before routing, verified AWS Listener ownership adds a soft preference toward its
LB. The provider resolves a complete, unique `load_balancer_arn` reference;
logical-component membership does not drive geometry. A bounded search moves only
the Listener card inside its existing parent, deployment panel or module band.
It scores owner distance and proximity to competing owners at four times ordinary incident-peer distance, retaining
Target Group connections. Existing connection-affinity members remain protected.
Candidate positions reserve marker-aware padding and keep the packed envelope;
when no improving legal position exists, the original position remains. Cards are
processed by owner and resource identity, and any height refinement repeats placement
before rerouting. Ownership never creates containment or moves the owning subtree.

The same bounded search gives ordinary noncontainer leaf cards a weaker preference
toward their one distinct visible spatial neighbor. Dependency, Connection and
Association edges count; parallel edges to one peer count once, while containment,
self-loops, logical resources and relationships removed by projection do not count.
Zero-neighbor and multi-neighbor cards keep their packed positions. A two-card
component anchors the lower stable identity; longer chains move only degree-one
ends. Multiple leaves move in peer/resource identity order without moving their hub.
Explicit owner cards and connection-affinity members remain protected.
Cross-container leaves stay within their existing parent and can move toward its
peer-facing boundary. Flat module bands retain rank columns and bundle routing,
adjusting only vertical positions. No legal improvement means no movement; this
pass adds no whitespace or containment and does not promise global optimality.

With containers, direct children occupy columns based on non-containment
relationships; descendant relationships influence sibling ranks. Cycles share a
column. Dense ranks wrap into adjacent columns, and containers grow around their
children. Related subnet containers can be kept adjacent to shorten connections.
Module membership does not control containment or create module bands.

After packing, up to four forward/backward sweep pairs order each physical
column by neighbor barycenters and align its stack with the median peer offset.
Only a strict reduction in total vertical peer distance is accepted; ties retain
the deterministic initial order. Columns, sizes and parents stay fixed. Descendant
edges project to their direct sibling owners, including external peers at their
common enclosing scope. Repeated relationships retain their weight. Connection
affinity protects only physical columns containing its sources or targets; other
columns in the same container still refine. Root protection includes the affinity
owners and resources directly attached to affinity members, not their entire
connected component.

Packing evaluates at most 65 height limits. The compactness baseline minimizes
`max(10 × width, 17 × height)`, then area. Candidates may exceed that baseline's
span by at most 10% and area by at most 25%; these bounds allow nearby shapes
without exchanging a compact container for a long strip. Within those bounds,
the objective is Manhattan distance between projected peer centers plus the
router's one-pixel crossing penalty for intersecting straight-line estimates.
Repeated relationships weight both signals. Compactness breaks routing-cost ties.
Wrapped headings and reference content expand card height before packing;
reference content can also expand card width. Containers and scope panels enclose
those actual bounds. Flat rank columns reserve the widest
card in each rank, and routing ports use the expanded borders.
The hub regressions cover useful shape changes and the shape bounds under heavy
relationship weights; variable-height cases use actual card centers.
Crossing estimates sample at most 256 distinct segments in geometric order,
keeping evaluation bounded and independent of input order. Full routing runs
only after placement. Affinity scopes use the same relationship objective, adding
target distance from the source median. Each candidate aligns targets into free
space before scoring. Coherent source runs cannot split across physical columns;
overlapping groups share that constraint. Compactness bounds use the constrained
baseline. Alignment preserves padding and containment without provider-specific rules.

Without containers or account/region scope panels, resources use module bands and dependency ranks. Long edges
participate in ordering through virtual intermediate vertices, which are not
rendered as cards. Numbered relationships retain these bands; rows accommodate
the actual card heights. Address labels, action metadata and provider roles do not change
during layout.

After initial routing, left and right endpoint demand determines each card's
height: the larger side wins, with 16px ordinary slots and 24px numbered slots,
plus edge padding and the minimum content height. Opposite sides share the same
vertical span. One deterministic refinement repacks containers and flat rows,
then assigns geometry-ordered slots on the selected sides and reroutes. Side
choices remain fixed for ordinary endpoints; numbered endpoints can reconsider
legal alternatives against the updated bounds and occupied slots. Card width does not expand for
top/bottom demand. If the requested top/bottom slots cannot fit, those endpoints
move to a facing left/right side before the single refinement and its height
calculation. All refined sides reserve marker-aware minimum spacing.
The initial top/bottom capacity check counts coincident ordinary terminals once
per node, side and endpoint role, preserving valid fan-in/fan-out bundles.
If another constraint requires refinement, capacity is recalculated for
independent branches before repacking, since nested refinement removes bundling.
Rank-lane bundles retain their alignment; refined nested
routes use independent paths and reject numbered alternatives that exceed the
available side capacity.
Ancestor/descendant routes also retain their allocated sides during refinement;
the container-boundary shortcut does not reselect their endpoints afterward.

Structural keys make ordering deterministic under input reordering. Symmetric
resources use complete Terraform identities as a final tie-breaker; renaming a
module can therefore change the order of otherwise indistinguishable resources.

## Paths and attachment

Flat routing uses rank lanes and compatible fan-in/fan-out bundles. Nested routing
uses recursive packing and obstacle-aware search, with shared gutters for compatible
subnet connections and Dependency fan-in/fan-out groups. Dependency groups share a
source or target and peer containment parent; unrelated sibling scopes remain separate.
Every branch is validated against its own hard boundaries. Resource-backed numbered
dependencies remain independent. Flat diagrams with numbered relationships also use obstacle-aware
search to preserve the reserved terminal corridors. Bundled edges retain individual metadata and junction markers.
When a relationship has numbers at both ends, packing reserves space for both
terminal clearances between adjacent cards so facing ports remain usable.
Connection and Dependency bundles use the same facing-side candidates and explicit ports as
independent routes, supporting horizontal and vertical trunks. Unsafe or more
expensive bundles fall back to independent paths; eligible groups need not bundle.
For Dependency groups, each eliminated duplicate terminal reduces bundle cost by
one bend (96), allowing a modest distance increase for a clearer common junction.
All original graph edges, arrows and metadata remain separate.
Each side uses its assigned incident slots. The cost baseline uses the independent
router and its shortcut pass, regardless of the bundle's direction. It evaluates
the group against paths available at the decision point, then restores the routing
index without copying previously routed paths. Later edges still undergo normal
routing and final shortcutting; bundle selection remains a local decision.
See [bundling](../examples/bundling.svg) and [spanning connections](../examples/spanning-dense.svg).

Final attachment occupancy is checked by node and exact point across both endpoint
roles. Exact source/target coincidences trigger refinement even when ordinary
spacing checks would ignore a shared terminal. Same-role bundle sharing remains
allowed. Flat bundle alignment checks other incident endpoints before choosing a
shared offset, prefers a free allocated slot when the center is occupied, and
falls back to independent paths if no candidate has sufficient spacing.
The [mixed-port regression](../examples/port-collisions.svg) reproduces a refined
fan-out bundle whose old central source coincided with an independent target.
Its manually positioned cards exercise geometry near the diagram's top edge.
SVG composition measures the final cards, paths and number badges and reserves
top clearance so they remain below the preceding legend and annotation sections,
without changing relative routing geometry.
The [shared SVG update command](../CONTRIBUTING.md#svg-の更新) regenerates it by selecting
`refined_flat_bundle_keeps_opposite_role_port` and reusing its exact fixture.

Independent paths compare route quality on actual bounds, preferring facing
side pairs when quality is equal. Rectangle projections first select one facing
pair for horizontal/vertical separation or two for diagonal separation, ordered
by port-distance and bend lower bounds. Remaining side pairs are generated only
if the primary routes fail to attain the rectangle-distance lower bound; overlapping
bounds use the broader search immediately. Both numbered and ordinary endpoints
use this progression, with the same hard boundaries, soft-card cost, occupancy
cost and detour budget. The old endpoint pair has no cost preference. Pairs whose
bounds exceed the best eligible route's cost are skipped. Simple paths that attain
the direction-aware cost lower bound without an upper-peer detour need no grid
search; the same check after shortest-path search can skip occupancy-aware search.
Ordinary nested edges select from their assigned ports directly, avoiding an
eager legacy-route search that side selection would repeat. The legacy route is
computed only if no valid side candidate exists.
Outward stubs and distinct endpoint slots protect labels and high-degree
ports. Endpoint and ancestor headers remain hard obstacles; unrelated containers,
including their headers, are visual occluders hidden by their per-edge SVG mask.
Strict endpoint ancestors permit traversal outside their headers. An endpoint
container is a full obstacle when its peer is outside it, protecting its interior
through side selection, independent routing, bundles and final shortcuts. A
container-to-descendant relationship keeps its header-only barrier and may attach
from inside. This applies symmetrically to incoming and outgoing routes and does
not rely on crossing penalties to discourage re-entry. The containment tree
determines these scopes. In nested routing, ordinary peer cards and unrelated
containers are soft occluders: each hidden pixel adds two pixels of cost for cards
and one per two-pixel interval for containers. Container intervals use absolute
coordinate division, making costs additive across grid subdivisions. Containers can cover long stretches; the lower weight
allows substantial bend/distance savings while still preferring short clear detours.
Nested occluders and their cards are counted only once, using the outermost foreign
container. Short detours can avoid them, but long detours can pass behind them. Search, candidate
comparison and shortcutting use this same policy. Endpoint cards remain protected. Connections
between a container and its descendants can attach from inside to its boundary
or the underside of its header, without visiting a child as an endpoint.
The [hierarchy regression](../examples/hierarchy-routing.svg) is generated by
`hierarchy_obstacles_and_direct_container_endpoints`. The
[shared SVG update command](../CONTRIBUTING.md#svg-の更新) selects that test with its snapshot
update flag, alongside the mixed-port fixture and all JSON-backed samples listed in
[`scripts/generated-svgs.txt`](../scripts/generated-svgs.txt).

Incident slots use peer center coordinates: vertical order on left/right sides,
horizontal order on top/bottom sides. Stable identities break geometric ties.
Baseline and side-aware candidates share this ordering and marker-aware allocator.
The assigned offset is the initial candidate; changing a side uses its projected
slot. Numbered endpoints also compare alignment with either opposite endpoint or
their midpoint, bounded to four offset pairs per side pair. Every alternative
must preserve corner margins and separation from other attachments. Top/bottom
slots account for each marker's width, and left/right slots for its height.
Intentional bundle alignment and numbered terminal spacing remain constraints.
After final height refinement and shortcuts, a bounded pass tries at most 32
adjacent same-side port exchanges on independent resource cards whose actual
paths cross. It reroutes both edges with their roles and opposite endpoints fixed,
protecting cards, container boundaries, headers, endpoint occupancy and numbered
corridors. Only fewer final crossings with no increase in overlap, total length
or bends is accepted. Marker spacing is revalidated after each exchange. Shared
terminals and paths through bundle junctions are excluded; container attachments
are not permuted. Peer-center ordering remains the initial allocation, while
this final pass handles approaches that disagree with that projection.
Numbered relationships share their provenance grouping, stable numbers and badge
geometry with SVG rendering. A badge is 18px high and `8 × digits + 16` pixels
wide. Each terminal reserves a straight approach of `10 + width + 8` pixels on
left/right sides and `10 + 18 + 8 = 36` pixels on top/bottom sides. The 10px gap
keeps directed arrowheads clear; the lateral reservation is the actual badge
dimension plus 6px. A single-digit terminal therefore keeps its own dimensions
even when other relationships have two or three digits. Numbered embedded Route
sources reserve the same badge-sized approach as other numbered Association
endpoints; unnumbered Route sources retain a 20px escape. Packing padding
still uses the largest displayed number conservatively; terminal validation does not.

Numbered relationships compare their lowest-cost valid sides, port alignments and
badge corridors together. Candidates are rejected when badges overlap cards,
protected headers or other reserved terminals. Parallel relationships can choose
the same side pair while retaining distinct slots. After the initial selection,
one additional sweep reconsiders at most 32 non-straight numbered relationships
against the settled reservations. The same bounded alternatives are available
after height refinement, including previously selected sides. Candidate lower
bounds and the existing detour budget limit search; initial sides have no priority.

Numbered ports at both ends of Associations and other undirected relationships,
or the destination of directed Dependencies, reserve badge-sized slots; other
incident ports retain normal spacing. Adjacent terminal corridors are merged
before routing. Orthogonal
search indexes blocked grid edges and soft-card occlusion costs once per search,
then looks them up in constant time for each candidate segment. Bundle branches
use their own endpoint exclusions when evaluating soft-card occlusion.
Occupancy-aware search also memoizes each grid segment's conflict cost and each
grid point's directional junction cost, shared by repeated search states and
reverse traversal. These caches borrow the fixed scorer, use memory proportional
to the search grid, and are discarded after that search.

Hard obstacle violations are rejected. Readability cost is path length plus 96 per
bend, one per crossing, eight per overlapping pixel, two per occluded card pixel
and one per two-pixel occluded container interval.
Visible frame proximity uses a 4px minimum empty gap and an 8px preferred gap,
separate from line-to-line spacing. Empty gap subtracts half of both stroke widths
from the centerline distance. Dependency/Containment, Association and Connection
strokes are 1.5px, 2px and 2.5px; ordinary cards, containers and embedded Route
cards use 1.5px, 2px and 1px borders. Inline Route entries inside table containers
are indexed at their rendered bounds even without separate graph nodes.
Routing and SVG rendering share these widths
and their corner radii. Distance is measured in quarter pixels, including rounded
frame ends and a conservative bounded evaluation of final quadratic path corners.
Short parallel contacts count without a 64px cutoff. Legitimate perpendicular
crossings and endpoint approaches remain allowed.

Each visible projected pixel adds its minimum-gap deficit in quarter pixels,
plus its preferred-gap deficit divided by 16 and rounded up. The finite preference
above 4px is deliberately weaker than an extra bend. Absolute projection intervals
keep straight-segment costs additive when search subdivides them. Card fills and
foreign-container masks exclude only covered path pieces; a visible line just
outside a masked container still has a measurable gap to its border. Search,
bundle comparison, shortcutting and final spacing use this same policy. Candidate
tracks lie inside or outside nearby frames at the minimum and preferred distances;
at most 32 coordinates per axis are added within 128px of the endpoint rectangle.
A single perpendicular crossing cannot pay for an extra bend or even a two-pixel
orthogonal detour. Equal-length, equal-bend alternatives prefer fewer crossings;
collinear overlap remains a separate, stronger penalty. Search segment and junction
costs use the same crossing weight as final scoring and shortcutting. Search,
candidate comparison and shortcutting share these weights. Straight, one-bend and two-bend candidates compete with grid-search
paths under the same safety and endpoint-direction checks. Occupancy-aware alternatives
are limited to six additional bends and twice the reference length plus 512 pixels
relative to the simplest valid candidate across the evaluated ports. This keeps numbered
terminal reservations from inducing excessive congestion detours. These alternatives
discourage crowded corridors. Selection prefers the lower corridor for otherwise
equal upper/lower detours around blocking peers in the same
containment layer; obstacle and edge-conflict quality remain authoritative. Selection
is deterministic but local, so it does not guarantee a global optimum or a crossing-free
diagram. Named-state edges use shared search with an external state gutter.

After all routes are fixed, independent routes remove safe orthogonal
doglegs while retaining both attachment points and endpoint directions. Shortening
does not feed back into endpoint selection or routing. Each shortcut is checked
against all other completed paths, must avoid obstacles,
reduce weighted readability cost and reduce bends or length without increasing either.
Numbered routes retain badge-sized terminal lengths and protect their badge corridors;
their middle segments can be shortened. Longitudinal clearance and lateral badge
protection remain separate through refinement, shortcuts and final spacing.
Shortcuts cannot introduce a gap below 4px or worsen an existing sub-4px minimum,
and cannot increase the length-weighted minimum-gap deficit.
Only the legitimate terminal approach may traverse its own reservation; the
opposite terminal leg and middle segments cannot enter sideways or re-enter it.
Bundles retain their shared geometry.

The final spacing pass indexes horizontal and vertical segments by their line
coordinate. Independent runs with at least 64px of overlapping projection target
16px centerline separation; the first/last 32px at real endpoints may converge.
Perpendicular crossings and disjoint projections do not contribute. Coincident
prefixes/suffixes connected to a common same-role terminal are shared trunks,
but branches beyond their divergence are measured independently.
The same pass also creates corridors from a single path's visible frame contacts,
including contacts shorter than 64px and quadratic corners with only a preferred-gap
deficit. Urgency comes from each contact's measured minimum gap, not the whole path.
Attempt identities include projection ranges so disjoint runs on the same track
remain separate. Fully masked frame constraints do not
reserve extra capacity; visible exterior contacts still do.

Corridors rank by their estimated finite deficit cost: the group's pairwise
line-spacing deficit divided by four, plus each movable edge's current border
cost counted once. The border term covers the whole edge, so the estimate can
include contacts outside that corridor. Sub-4px gaps retain their stronger shared
penalty. Equal estimates prefer minimum-gap violations, then border contacts,
then stable geometric and resource order. Groups with no independently movable
lane, repeated groups and previously attempted groups are filtered before the
remaining attempt budget is applied. Until an urgent corridor has been attempted,
the highest-ranked urgent border group moves into the final available slot if it
would otherwise fall outside that budget. This reserves at most one of the remaining
attempts, preserving the higher-ranked prefix; an urgent attempt consumes the
reservation even if its change is rejected. Every accepted change must still
satisfy the minimum-gap guards below.

Connected groups of overlapping runs receive ordered lanes over their combined
corridor before any route is changed. Geometric order and resource identity break
ties. A line-to-line corridor reserves the lane span plus 16px on each side.
A border-only corridor instead reserves its actual stroke-dependent preferred
margin, usually 10px or 11px between centerlines. If capacity is
insufficient, a legal inter-card gap can grow by the missing amount. Cards beyond
the cut move together; enclosing containers, scope panels and bands grow without
scaling resource cards. Cuts cannot split cards or headers. Terminal locations
move with their card boundaries, retaining side and order.

Impacted independent edges compare new routes and translated seeds against the
updated card/header obstacles, terminal reservations and lane preferences.
Whole straight, L-shaped and corridor routes compete. Corridor alternatives use
the assigned lane and the midpoint between outward terminal stub coordinates,
plus nearby minimum/preferred frame tracks and tracks 16px to either side of
actual parallel peers. Peer tracks require at least 64px of overlapping corridor
projection and stay within the endpoint coordinate interval plus 128px on either
side. Optional tracks rank by projected deficit against actual peer runs, then
distance to the assigned lane or midpoint, then coordinate. The assigned lane
and midpoint lead the list when inside the canvas; the combined list is
deduplicated and capped at 32 coordinates. Moving an existing run moves its two
corners together while retaining the other runs. Expansion and border-only
repairs can also invoke bounded search with these selected coordinates.

A fresh peer index for each sequential reroute uses the other paths' actual
positions, including protected routes that retain their original tracks. Each
completed candidate adds its incident line-spacing deficit divided by four to
readability cost after the existing shape comparison. This deficit uses the same
64px projection threshold, 32px endpoint allowance and shared-trunk exclusions as
the final spacing metric. Grid search retains its additive segment costs.
Visible ordinary cards remain protected during final spacing; supported foreign
container masks retain their routing role. Completed candidates, including
translated seeds, pass the same shortcut, validation and final quality checks.
Intentional bundles and junctions retain their shared geometry under
the same monotone gap transformation. A failed or non-improving transaction rolls
back bounds, paths, junctions, scopes and bands together.

Acceptance compares each final edge's bends, short perpendicular offsets, axis
reversals and offset/re-entry S-jogs against its original shape. None may increase.
An immutable copy of the original paths receives only accepted right/down gap
transformations, never replacement with a newly routed path. Its transformed
length and backtracking bound candidates in the updated endpoint/obstacle
geometry, with 128px of length slack for the entire pass. This linear reference
update permits necessary endpoint separation and obstacle growth without an
expensive shortest-path comparison or accumulating detour allowances.
A short offset is measured
relative to its neighboring legs, not treated as universally invalid near a port.
Exact overlap cannot increase on rerouted edges. Whole-route clarity takes priority
over the combined line-spacing and border-clearance cost, with excess length
above current endpoint Manhattan distance and
expansion cost breaking otherwise equal shape comparisons. An accepted change
cannot introduce a sub-4px gap or worsen an existing sub-4px minimum on any edge.
Each edge's length-weighted minimum-gap deficit must also remain unchanged or
decrease. These comparisons use the bounds and paths captured before the
transaction; geometry growth cannot hide a regression in another route.
Preferred gaps are soft: a clean residual near-parallel run beats new bends or an
excessive detour. When preferred capacity cannot be obtained, the pass tries the
minimum stroke-aware margin within the existing geometry.

This pass runs once on the final graph, after embedded Route and cluster-member reconstruction,
same-side port refinement, bundling and ordinary shortcutting. It permits at most
20 sweeps and `clamp(path_count, 60, 240)` total corridor attempts. Cumulative
growth on each axis is `clamp(initial_axis_size / 4, 640, 4096)` pixels, shared by
border and line-to-line repairs. Allocation uses only actual lane and border
demand. Limits of 16 lanes per corridor and 32 rerouted edges per attempt remain.
Each accepted transaction ends its sweep and rebuilds demand from current bounds
and paths. A failed key is excluded for that unchanged state, then becomes
eligible after an accepted geometry or peer-routing change. Reconsiderations
consume the total attempt budget. A sweep with no improvement stops early.
Urgent border demand retains its reserved place within remaining attempts.
Corridors with no independently movable lane are skipped before consuming an
attempt. Translated non-corridor paths retain their shape when legal; changed
paths are revalidated together for ports, protected headers, numbered terminals,
overlap and border regressions. Invalid translated paths join rerouting, and a
failed candidate restores the whole transaction. Unchanged line-only geometry uses whole-path candidates
with the same bounded frame and peer tracks. Regression metrics disclose
residual close length, visible minimum gap and both minimum/preferred deficits;
deterministic fallback preserves valid
routes when capacity, obstacles or budgets prevent improvement.

See the [budget calibration](../examples/terraform-large/spacing-calibration.md)
for previous-limit and doubled-candidate comparisons and CI artifacts.

SVG rendering consumes the final geometry and rounds corners without changing
relationship identity. Nested containment edges receive no redundant path.
Paths use shared, containment-specific luminance masks to hide foreign container
fills, rounded borders and descendants, with two pixels of stroke clearance.
An endpoint's ancestor containers are excluded from its mask, preserving internal
relationships. Junction dots use their participating route's mask; endpoint circles
and numbered badges retain their foreground layer and reserved corridors.
Mask IDs and groups sort by resource identity and are namespaced in named states.

## Verification

Tests cover hierarchy, variable-sized bounds, dense routing, bundles, input/module
reordering and SVG regressions. `src/layout/metrics.rs` measures pairwise
overlap, internal orthogonal crossings, bends and length; endpoint contacts and
self-crossings are excluded. Run the [Docker checks](../CONTRIBUTING.md#検証).
The large fixture also checks every visible edge: at most 14 bends and a length
at most twice its shortest valid reference plus 128 pixels.
The reference keeps the rendered ports and terminal clearance, protects container
endpoint boundaries, ancestor headers and numbered corridors, and ignores occlusion and edge-conflict
costs. Its rectilinear search includes exact obstacle boundaries, so necessary
container detours are included in the baseline. Failures name both endpoint
addresses and report bends, length, reference length, excess distance and stretch.
The test counts extreme routes and verifies identical metrics after edge reordering;
aggregate quality checks remain in place.
The large fixture records every edge's shape and compares the subnet-to-private
Route Table, VPC Endpoint and ECS routes with a reference that also protects foreign
cards and containers. This distinguishes required marker/header detours from
avoidable steps. The [per-edge comparison](../examples/terraform-large/routing-comparison.md)
records initial-PR and revised results, including residual violations and runtime.
The [#317 calibration](../examples/terraform-large/README.md#317-calibration)
compares the #322 baseline and final SVGs using the same border metric, including
the resulting changes in bends and crossings.
The crossing policy replaces older tests that demanded crossing-free detours:
they now require short crossing routes, including at stub/grid junctions and after
shortcutting. Hard obstacles, overlap avoidance and terminal reservations remain tested.
The large-fixture regression checks short subnet-to-ECS paths, a short IGW-to-NAT
path with limited hidden length, and zero visible straight-frame overlap. The
gateway limit is its shortest valid route plus 128 pixels and at most three bends,
so larger content-driven cards do not fail an obsolete absolute-distance cap. These
checks use the current card geometry and repeat after edge reordering; they do
not require a path to pass behind a particular container.
Performance measurements are documented in [benchmarks](benchmarks.md).
Opt-in [layout diagnostics](layout-diagnostics.md) report the final selected
diagram, visible spatial findings and explicitly labelled raw-polyline metrics.
The single-path border fixture covers all three relationship widths, both axes,
and 32px/300px frame spans. A 12px card gap grows by 8px, followed by a coherent
two-bend reroute into the resulting 20px corridor. A protected-junction variant
keeps its original bounds, paths, scopes and bands with measurable residual
proximity. Border tests cover 0/1/2/4/8px offsets, rounded corners, masked path
pieces and additive search subdivision.
Protected-peer fixtures verify that both routing orientations use the peer's
actual position while preserving its path and junction. Bounded-track fixtures
retain useful alternatives across the endpoint interval and keep the same
32-coordinate selection after peer reordering or adding distant paths.
Thread-local test counters verify that unobstructed horizontal/vertical side
selection evaluates one pair without grid search, while blocked ports expand the
search. Focused cases compare selected quality with exhaustive side evaluation.
