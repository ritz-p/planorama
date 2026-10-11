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

## Placement

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
common enclosing scope. Repeated relationships retain their weight. Scopes with
special connection affinity keep their existing grouping and alignment instead.

Packing evaluates at most 65 height limits. The compactness baseline minimizes
`max(10 × width, 17 × height)`, then area. Candidates may exceed that baseline's
span by at most 10% and area by at most 25%; these bounds allow nearby shapes
without exchanging a compact container for a long strip. Within those bounds,
the objective is Manhattan distance between projected peer centers plus the
router's 512-pixel crossing penalty for intersecting straight-line estimates.
Repeated relationships weight both signals. Compactness breaks routing-cost ties.
The hub regressions cover useful shape changes and the shape bounds under heavy
relationship weights; variable-height cases use actual card centers.
Crossing estimates sample at most 256 distinct segments in geometric order,
keeping evaluation bounded and independent of input order. Full routing runs
only after placement. Special affinity scopes retain compact-only selection
because straight-line estimates do not model their shared connection routes.

Without containers or regional scope panels, resources use module bands and dependency ranks. Long edges
participate in ordering through virtual intermediate vertices, which are not
rendered as cards. Numbered relationships retain these bands; rows accommodate
the actual card heights. Address labels, action metadata and provider roles do not change
during layout.

After initial routing, left and right endpoint demand determines each card's
height: the larger side wins, with 16px ordinary slots and 24px numbered slots,
plus edge padding and the minimum content height. Opposite sides share the same
vertical span. One deterministic refinement repacks containers and flat rows,
then assigns geometry-ordered slots on the selected sides and reroutes. Side
choices remain fixed during this refinement; card width does not expand for
top/bottom demand. Rank-lane bundles retain their alignment; refined nested
routes use independent paths to keep the selected per-side demand fixed.
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
ports. Unrelated container interiors and all container headers are hard obstacles;
endpoint ancestors permit traversal outside their headers. The containment tree
determines these scopes. In nested routing, ordinary peer cards are soft occluders:
each pixel hidden behind a card adds two pixels of routing cost. Short detours can
avoid them, but long detours can pass behind the opaque cards. Search, candidate
comparison and shortcutting use this same policy. Endpoint cards remain protected. Connections
between a container and its descendants can attach from inside to its boundary
or the underside of its header, without visiting a child as an endpoint.
The [hierarchy regression](../examples/hierarchy-routing.svg) is generated by
`hierarchy_obstacles_and_direct_container_endpoints`; set `UPDATE_ROUTING_SNAPSHOT=1`
when intentionally updating that test's SVG.

Incident slots use peer center coordinates: vertical order on left/right sides,
horizontal order on top/bottom sides. Stable identities break geometric ties.
Baseline and side-aware candidates share this ordering and marker-aware allocator.
Keeping a side preserves the existing offset; changing a side uses its projected
slot. Top/bottom slots account for marker width, and left/right slots for height.
Intentional bundle alignment and numbered terminal spacing remain constraints.
Numbered relationships select their lowest-cost valid sides before routing and reserve their final
badge corridors. Candidates are rejected when badges overlap cards, protected
headers or other reserved terminals. Parallel relationships can choose the same
side pair while retaining distinct slots.

Numbered ports at both ends of undirected associations, or the destination of
directed relationships, reserve badge-sized slots; other incident ports retain
normal spacing. Adjacent terminal corridors are merged before routing. Orthogonal
search indexes blocked grid edges and soft-card occlusion costs once per search,
then looks them up in constant time for each candidate segment. Bundle branches
use their own endpoint exclusions when evaluating soft-card occlusion.
Occupancy-aware search also memoizes each grid segment's conflict cost and each
grid point's directional junction cost, shared by repeated search states and
reverse traversal. These caches borrow the fixed scorer, use memory proportional
to the search grid, and are discarded after that search.

Hard obstacle violations are rejected. Readability cost is path length plus 96 per
bend, 512 per crossing, eight per overlapping pixel and two per occluded card pixel.
Thus a minor crossing improvement cannot justify an unlimited detour. Search,
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
their middle segments can be shortened. Bundles retain their shared geometry.

SVG rendering consumes the final geometry and rounds corners without changing
relationship identity. Nested containment edges receive no redundant path.

## Verification

Tests cover hierarchy, variable-sized bounds, dense routing, bundles, input/module
reordering and SVG regressions. `tests/support/layout_metrics.rs` measures pairwise
overlap, internal orthogonal crossings, bends and length; endpoint contacts and
self-crossings are excluded. Run the [Docker checks](../README.md#開発・検証).
The large fixture also checks every visible edge: at most 14 bends and a length
at most twice its shortest valid reference plus 128 pixels.
The reference keeps the rendered ports and terminal clearance, protects container
boundaries, headers and numbered corridors, and ignores soft-card and edge-conflict
costs. Its rectilinear search includes exact obstacle boundaries, so necessary
container detours are included in the baseline. Failures name both endpoint
addresses and report bends, length, reference length, excess distance and stretch.
The test counts extreme routes and verifies identical metrics after edge reordering;
aggregate quality checks remain in place.
Performance measurements are documented in [benchmarks](benchmarks.md).
Thread-local test counters verify that unobstructed horizontal/vertical side
selection evaluates one pair without grid search, while blocked ports expand the
search. Focused cases compare selected quality with exhaustive side evaluation.
