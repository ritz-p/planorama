# Presentation geometry checks

The [Layout quality workflow](ci.md#layout-quality-comparison-390) supplements
the Rust routing diagnostics with `scripts/presentation-quality.cjs`. It checks
the generated SVG in Chromium, without fetching fonts, icons or remote services.
It uses the renderer's frames, semantic attributes and shared measurements;
it does not perform a second layout.

```sh
docker compose exec -T dev cargo build --locked
docker compose exec -T dev node --test tests/browser/presentation-quality.test.cjs
docker compose exec -T dev node scripts/presentation-quality.cjs output/presentation --maintained
# Or inspect selected freshly rendered SVGs:
docker compose exec -T dev node scripts/presentation-quality.cjs output/presentation output/diagram.svg
```

`--maintained` reads `scripts/generated-svgs.txt`. Regenerate those files with
the [normal procedure](../CONTRIBUTING.md#svg-の更新) before evaluating a renderer
change. This command checks the files on disk; it does not silently regenerate
or accept findings. CI checks the six freshly rendered layout fixtures and
focused rendering cases covering spatial cards, container headers, embedded
members/routes, Logical Resources action sections, Logical Components and
Relationship Resources. Long types/addresses, multiple actions, data/import
indicators, narrow frames and named states are exercised in browser tests.

## Findings and bounds

`presentation.json` records each input's status, browser version, configured font
families, collection counts and completeness, and actionable findings. Elements
include resource address or architecture ID, named state, resource-type group,
relationship entry number, section, kind, text
where applicable and bounds. `presentation.md` is the CI summary. Exit status
is nonzero for findings, incomplete analysis, missing geometry or input errors.
There is no baseline allowance: a clean fixture must remain clean.

- `overlap`: text/text, text/badge or badge/badge intersection.
- `content-overflow`: text or badge exceeds the nearest enclosing frame's
  interior, including the actual action section or embedded child frame.
- `header-overflow`: content exceeds a container header's allocated area.
- `badge-text-overflow`: a badge's own text exceeds its interior and padding.
- `badge-frame-intersection`: a badge crosses a visible frame border.

Wrapped `tspan` lines are checked separately. Browser `getBBox()` bounds are
transformed by the SVG CTM, including named-state translations and scale. Rect
strokes expand painted bounds and reduce content interiors. Service label
padding and container header areas come from the same Rust measurements as
rendering (`data-presentation-padding` and `data-presentation-header`). Text
inside its own badge is intentional but remains subject to containment checks.
Nested frames, resource/member underlines and selection decorations are not
text collisions. Relationship endpoint markers are routing decorations and
are excluded; their copies in Relationship Resources are checked.

The numerical tolerance is 1 SVG viewport pixel on each axis. An intersection
must exceed this on both axes. Sweep-line candidates use vertical overlap;
analysis stops after 2,000,000 candidate comparisons, retains at most 20,000
findings and reports either limit's incompleteness as failure. The checker does not enumerate every pair
across a large diagram. Badge ownership and linked container frames use maps.

## Limitations

This report is explicitly **browser-observed bounding-box geometry**. The Rust
[layout report](layout-diagnostics.md) remains a separate measurement-based
routing approximation; its schema and baseline contract are unchanged.
Header allocation and label padding are layout metadata, not independently
measured glyph ink. Chromium's font metrics and configured fallback fonts can
differ from another browser or operating system. CI uses Debian Bookworm's
Chromium and DejaVu fonts, matching the Docker development environment; package
updates can change measurements. The report records the browser and configured
families, not the actual fallback face selected for every glyph.

Axis-aligned bounding boxes approximate rotated/skewed text, rounded corners,
glyph ink and stroke joins. Clipping, opacity and custom CSS effects are not
modeled. The check targets generated static diagrams, not arbitrary SVG or
interactive viewer states. Canvas titles, legends, routing paths and artwork
are outside this focused card check. A passing result is not proof of perfect
typography on every system; review representative rendered output when changing
fonts or geometry. Keep ordinary rendering, routing and browser tests enabled.
