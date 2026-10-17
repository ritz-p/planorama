# Final layout diagnostics

`--layout-diagnostics` writes an informational text report to stderr.
`--layout-diagnostics-json PATH` writes the same measurements to a separate JSON
file. Both can be combined with semantic `--diagnostics`, SVG stdout, HTML,
filters and named states. JSON never uses stdout or stderr. Input, diagram and
report files must differ. Findings do not change exit status; I/O failures do.
Dangling output symlinks are rejected before either output is written.
The JSON file is opened without truncation, then its filesystem identity is
checked against the diagram before writing either payload. Case-only aliases on
case-insensitive filesystems are rejected; a newly reserved empty file may remain.
`--html-diagnostics` also enables collection and embeds a bounded diagnostic
bridge when used with `--format html`; see [routing hotspots](viewer.md#routing-hotspots).
It does not write a separate report unless one of the report options is provided.
Without these options, no report or pair analysis is collected.

```sh
docker compose exec -T dev cargo run --release --locked -- examples/terraform-large/plan.json -o output/diagram.svg --layout-diagnostics --layout-diagnostics-json output/layout.json
```

Create `output` first. For a comparison, run the same command at each revision,
save the reports as `before.json` and `after.json`, and compare `summary`,
`states`, and `edges` keyed by `id`. `git diff --no-index before.json after.json`
shows the complete deterministic report. Keep inputs, filtering, named state IDs
and address format identical. The output contains identities and geometry, never
Terraform attribute values. No shortest-route search or profiling runs as part
of collection.

## Schema 1

The root has `schema_version: 1`, `category: "layout"`, `scope: "whole_diagram"`,
`units`, `canvas`, `states`, `edges`, `summary`, `findings`, `policy` and `limits`.
Consumers should check the version before comparing reports.

- `canvas` is the full SVG width/height, including annotation sections and gutters.
- Each `states` entry has a nullable state ID, the architecture's canvas origin,
  unshifted `spatial_layout` dimensions, retained source-resource count, and
  separate spatial, embedded-route, logical-resource, logical-group,
  logical-component and relationship-group counts. Groups are not resources.
  `embedded_members` is present when RDS cluster member cards are displayed;
  these members are excluded from spatial and embedded-route counts.
  Zero-sized logical placeholders are excluded from spatial frames.
- `edges` contains nonempty routed polylines, sorted by stable `id`. The ID is
  canonical JSON of the structured `identity`: state-qualified source/target,
  relationship kind/direction and provenance. Relationship resource identities
  distinguish coincident endpoints; no vector index is used as an edge ID.
  Cross-state identities include remote-state address and output name.
  Identical semantic relationships share identity; their geometry is not an ID.
- Edge and finding coordinates are in the whole SVG canvas. State offsets and
  cross-state gutter routes come from the same rendering operation. Filters,
  projection, embedded routes, port refinement, bundling, simplification and
  final spacing have already run. Logical cross-state references have no path.
- `raw_polyline` length and bends describe orthogonal centerlines before SVG
  corner rounding. Length uses px. Consecutive collinear segments are merged;
  zero-length segments do not contribute. Direction reversals count as bends.
  Unsupported diagonal segments are
  excluded and explicitly counted in `limits`.
- Pair findings include both stable IDs, zero-based segment positions within
  their reported polylines, and crossing coordinates or parallel intervals.
  Retraced collinear visible intervals are unioned for pair metrics; their segment
  position identifies the earliest contributing segment, and the location gives
  the measured interval. Raw length still counts every traversal.
  Merged overlaps are split at shared-prefix/suffix boundaries before classifying
  shared trunks and independent overlap; each visible interval contributes once.
  Border findings include a frame identity/bounds, segment, interval and whether
  the minimum gap is violated. Segment positions locate geometry, not identity.

## Visibility, thresholds and exclusions

Visible straight-section crossings count perpendicular interior intersections
once per edge pair and position. Endpoint contacts and self intersections are
excluded. Independent overlap measures intersecting continuous stroke envelopes;
shared source-prefix and target-suffix bundle trunks are reported separately.
Parallel proximity requires at least 64px after removing 32px endpoint convergence
zones, with centerline separation below 16px. Deficit is length times the missing
centerline separation in px². Perpendicular crossings do not contribute to it.

Pair geometry trims rounded corners and clips fully hidden sections using the
same rounded container masks and ordinary-card occlusion geometry as routing.
Integer clipping is conservative. Curve-to-curve and curve-to-line pair metrics
are unavailable; straight-section metrics must not be interpreted as exact
raster counts. Dash gaps, arrowheads, endpoint badges, text, annotation cards,
scope panels and module-band borders are excluded from spatial conflicts.

Visible border measurements share the runtime router's implementation: continuous
stroke widths, 4px minimum and 8px preferred edge-to-border gaps, rounded card and
route corners, applicable masks, quarter-pixel clearance rounding and 32 samples
per quadratic path corner. Deficits are px² and close length is px. Minimum gap
is null when no eligible border is within 8px; negative values mean overlapping
stroke envelopes. Perpendicular attachment at a straight card border does not
count as a parallel clearance violation. Cross-state square path corners use
the rounded-border approximation, disclosed in the report's policy.

Collection stops pair comparisons at 2,000,000 and retains at most 20,000 findings.
`limits` discloses incomplete pair metrics or findings; truncated totals must
not be treated as complete. Per-edge raw and border totals remain available.
Reaching a cap exactly is complete unless another comparison or finding is
omitted. Stopping pair analysis also makes the findings incomplete.
The CLI remains informational. The opt-in [CI quality comparison](ci.md#layout-quality-comparison-390)
applies fixture-specific budgets to these reports without changing CLI exit status.

The shared raw helpers are in `src/layout/metrics.rs`; the renderer's report
collector is `src/layout/diagnostics.rs`. Tests use these implementations, plus
focused crossing, overlap, shared-trunk, mask and border fixtures.

For optional overhead measurement, run:

```sh
docker compose exec -T dev cargo test --release --locked collection_overhead -- --ignored --nocapture
```

This measures composition with and without diagnostics using one final layout
of `terraform-large`, excluding layout and file I/O. Timings are observations,
not CI thresholds.

On 2026-10-14, `1920cd3` plus the layout-diagnostics instrumentation, Rust 1.85.1 release
in Docker Desktop Linux: two disabled runs took 3.386/3.308ms; enabled runs took
7.683/7.867ms. SVG remained 318,966 bytes and compact report JSON was 165,641
bytes. These same-layout observations include collection and report construction,
excluding JSON serialization and file I/O; other validation was running.
