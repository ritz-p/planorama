# Geometry-aware attachment

## Layout/routing integration (#82)

| Stage | Shared contract | Implemented in |
| --- | --- | --- |
| Derived hierarchy | `ContainmentTree`: unique parents, deterministic children/roots, inclusive ancestry, cycle/ambiguity handling | #83 `layout/containment.rs` |
| Geometry | `Bounds`, `Point`, `Side`, `Port`; actual card/container dimensions | #84 `layout/geometry.rs` |
| Routing primitives | Obstacle intersection, segment coverage, scoring, orthogonal search, explicit simplification policies | #85 `layout/routing_shared` |
| Orchestration | One hierarchy → placement → routing → final geometry pipeline; specialized strategies below stages | #86 `layout/pipeline.rs` |

`Layout::new` constructs the hierarchy once. Both strategies retain it in the
result; nested placement only computes geometry, and routing uses that same
hierarchy for endpoint ancestry and obstacles. Flat routing keeps rank lanes and
bundling; nested routing keeps recursive packing and endpoint-aware search. They
share geometry, occupancy and candidate scoring rather than becoming one oversized
router. Named-state margin routes also use the shared search.

The integration preserves semantic edges, containment and provider classification.
The extraction commits #83–#86 intentionally leave SVGs unchanged. #81 separately
introduces the visual changes described below. Regression coverage includes nested
roots/cycles, variable-sized bounds, crowded lanes, high-degree ports, spanning
bundles, module/input reordering, captured Terraform plans and checked-in SVGs.

Validation uses the Docker service:

```sh
docker compose exec -T dev cargo fmt --check
docker compose exec -T dev cargo clippy --locked --all-targets -- -D warnings
docker compose exec -T dev cargo test --locked
```

The final integration passes 321 tests; the opt-in benchmark is ignored in the
normal suite. Performance measurements are documented separately in
[benchmarking](benchmarks.md). Port-candidate evaluation adds search work;
the routing improvements are not a claim of faster execution.

## Attachment policy

Layout derives a containment tree once, places cards/containers, then routes
their relationships. Flat and nested strategies share bounds, four-sided ports,
obstacle intersection, segment occupancy, scoring and orthogonal grid search.

For independent paths, both routers compare their existing route with facing
port pairs: right/left for a target to the right, left/right for a target to the
left, and bottom/top or top/bottom for vertically separated bounds. Diagonal
separation may offer both orientations. Actual expanded container bounds are
used. Stubs leave the selected side outward; blocked candidates are discarded.
An already selected compatible bundle retains its shared route and junctions.

Selection is deterministic and prioritizes obstacle crossings, overlap, edge
crossings, bends and length in that order. Unrelated container interiors stay
obstacles; only the endpoints' containment ancestry is relaxed to header bounds.
Endpoint slots remain distinct even when projected onto a shorter side. A
candidate that does not improve the score leaves the original route in place.
This is local route optimization, not a global optimality guarantee.

The dense unit-test graph changes from 896 to 739 crossings and from 88,372 to
86,722 units of path length; bends change from 128 to 136. Overlap remains zero.
The corresponding deterministic regression was intentionally updated for #81.
The earlier refactors #83–#86 preserve output; #81 is the visual behavior change.

Named-state edges continue to use the shared orthogonal search and an external
state gutter. Their explicit state/output provenance and obstacle rules remain
separate from state-local port selection.
