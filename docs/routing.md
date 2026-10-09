# Geometry-aware attachment

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
