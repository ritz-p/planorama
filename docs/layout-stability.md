# Layout stability measurement

This opt-in developer experiment compares final resource frames across controlled
input and display-setting changes. It does not stabilize placement, establish a
regression budget, or run as a required CI check. Crossings, overlap, clearance,
routing quality and timing retain their separate measurements.

## Run paired generations

From the repository root, with the development container running:

```sh
docker compose exec -T dev cargo build --locked --release --bin planorama
docker compose exec -T dev node scripts/layout-stability.cjs target/release/planorama tests/layout-stability/pairs.cjs output/layout-stability
docker compose exec -T dev node --test tests/ci/layout-stability.test.cjs
```

The deterministic [paired fixtures](../tests/layout-stability/pairs.cjs) apply
explicit add/replace/remove patches to [a small nested input](../tests/layout-stability/base.json).
They cover identical settings, resource and relationship addition/removal, a long
move-provenance label with retained current identity, address format, legend
visibility, focus selection, logical-to-spatial presentation and named-state offsets.
Every pair retains before/after plan files, SVGs, final-layout JSON and
`comparison.json`; `summary.json` indexes the suite. Each generation is also run
without diagnostics and must produce identical SVG bytes.

The command accepts a version-1 JSON manifest or a trusted repository `.cjs`
fixture module exporting that manifest. Each pair specifies `id`, `description`,
`before`/`after` states and CLI settings, structured `affected` identities and
`intentional_restructure`. A state contains `name` (null for one unnamed input),
`input` and optional patches with `op`, array-valued `path` and `value`. Artifacts
are written only under the supplied output directory; input files are not modified.
Use a distinct ignored output directory. No Terraform commands or cloud access run.

To verify reproducibility:

```sh
docker compose exec -T dev node scripts/layout-stability.cjs target/release/planorama tests/layout-stability/pairs.cjs output/layout-stability-repeat
docker compose exec -T dev diff -r output/layout-stability output/layout-stability-repeat
```

Reports exclude timestamps, execution durations and output-directory paths. Their
input and SVG SHA-256 hashes identify compared bytes. A failure to generate,
unsupported report schema or nonzero identical control fails the command; measured
movement itself never fails a budget. Existing quality baselines are not refreshed.

## Interpretation

`comparison.json` has `schema_version: 1` and `category: "layout_stability"`.
Resources match by `{state, address, mode, deposed}`, preserving managed/data,
current/deposed and named-state identities. SVG order and visible labels are not
identifiers. Added/removed resources are separate arrays. Duplicate or malformed
identities are `unmatchable`; retained identities lacking comparable frames are
`unmeasured`. Hidden resources are removals, never retained zero-sized rectangles.

Each `matched` record contains before/after bounds, top-left displacement in whole
diagram coordinates, width/height/area changes and displacement relative to the
nearest shared containing resource, deployment panel or state architecture origin.
Scope identities match independently of their coordinates. If containment changes,
comparison falls back to a shared ancestor; no shared scope means null local
movement. Summary statistics include count, mean and maximum; empty populations
have null statistics. `largest_movements` and `largest_local_movements` identify
the ten largest whole-diagram and local movements with full context.

`uniform_translation_of_measured_resources` describes equal origin displacement
across all measured retained resources, not proof that sizes or all content stayed
unchanged. `declared_unrelated` means outside the fixture author's `affected` set;
it is not inferred causal independence. Declare relevant endpoints for relationship
changes. Global restructuring settings with no affected subset leave this
classification null. Intentional restructuring is recorded prominently in context
and suite summaries; it does not waive or hide measurements.

The initial fixture run observed zero movement for identical inputs and the short
address-format change. Adding an instance moved retained origins by up to 152px.
Adding a dependency moved them by about 664.05px globally and 390.77px locally.
Omitting the legend moved all measured origins by -558px vertically, with zero
local displacement. Lengthening a label in state `alpha` shifted state `beta` by
90px globally while beta's local positions stayed fixed. These are observations
for this fixture, not acceptance thresholds or performance promises.

## Geometry contract and limitations

The existing [final-layout diagnostic collector](layout-diagnostics.md) supplies
an additive `resource_geometry` object with its own `schema_version: 1`. Its
records use the final `Layout.bounds`, after placement, routing/spacing refinement
and embedded-resource allocation, translated by the same composition offsets as
the renderer. Scope origins use final containment and deployment frames. Only
effective selected-view resources are present; lowered identities come from
selected coverage accounting.

This first version measures spatial and embedded resource frames. Logical rows,
lowered relationship resources and synthetic component frames do not yet have
independent comparable spatial bounds in this collector. Logical/lowered retained
resources are explicitly unmeasured; synthetic groups are not Terraform resources.
Presentation changes crossing that boundary remain unmeasured, rather than being
reported as zero motion. Frames are layout rectangles, not glyph/pixel extents.
Rotation, route geometry, label internal movement and attribute values are not
compared. Existing structured edge diagnostics remain available in the paired
artifacts, but this metric does not match arbitrary SVG paths or mix endpoint
movement with routing changes.

Use large *local* movement of declared-unrelated resources to choose reproducible
cases for a later placement-stabilization issue. Investigate parent resizing,
containment changes and identity availability before drawing conclusions. Extending
logical/relationship frame coverage and structured route comparisons is separate
work; budgets should follow measured use cases and reviewed tradeoffs.
