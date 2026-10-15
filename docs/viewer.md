# Local HTML viewer

Use `--format html` to generate one file containing the SVG, resource index,
styles and JavaScript. Open it directly in a browser with JavaScript enabled;
no server, network, remote assets or runtime installation is required.

```sh
planorama plan.json --format html -o diagram.html
planorama plan.json --changed-only --format html -o changes.html
planorama --state app=app.json --state network=network.json --format html -o architecture.html
planorama plan.json --format html -o - > diagram.html
```

The default remains SVG and `diagram.svg`. HTML defaults to `diagram.html`;
`-o` sets the destination, not the format. Existing address formatting, focus,
changed-only selection, remote-state mappings and diagnostics work in both modes.
Logical Resources rows show the same per-resource deployment-scope labels in both
formats; see [scope evidence and limitations](aws.md#deployment-scopes).

The maintained [combined AWS viewer](../examples/multi-account/diagram.html)
exercises 250 input instances, repeated addresses in three states, region
backgrounds and remote logical/spatial references. Its [generation and review
commands](../examples/multi-account/README.md) use the same inputs as its SVG.

Search matches the complete native and qualified address, architecture identity,
module, state, resource type, managed/data mode and deposed key. Space-separated
terms must all match. Results retain full addresses and state/mode distinctions.
Selecting a result or diagram resource reveals its individual stable target,
including nested logical-resource rows and embedded Route cards.
Lowered association resources use their visible Relationship Resources entry;
their selection identifies the original relationship endpoints and paths.

Click a relationship number in the diagram or index, or focus it with Tab and
press Enter or Space, to select that exact relationship. Its paths, both endpoint
resources, matching badges and index entry are emphasized together. The selection
panel provides buttons to navigate to each endpoint. Selection uses the indexed
relationship identity, preserving parallel relationships, deposed identities and
equal numbers in different states. Address search continues to select resources.

Selection details separate incoming and outgoing rendered edges, undirected
relationships, Uses/reference targets, inverse references and Logical Component
membership. Links navigate only to targets present in the generated diagram.
Unavailable and unresolved reference rows remain explanatory text. References
and membership never become routed edges. Selection outlines resources and
emphasizes existing paths without changing direction, action color, markers,
line patterns, bundles or routing. Clear selection restores their normal styling.

| Operation | Controls |
| --- | --- |
| Search | Type in the search field; Enter selects the first result |
| Results | Arrow Down enters results; Up/Down and Home/End navigate; Enter selects |
| Resource links | Tab to a resource or link, then Enter; clicking also selects |
| Relationship numbers | Click, or Tab then Enter/Space; endpoint buttons navigate to resources |
| Clear selection | Escape or Clear selection |
| Pan | Drag or scroll; arrow keys while the diagram has focus |
| Zoom | +/− buttons, +/− keys in the diagram, or Ctrl/Command+scroll |
| Fit entire view | Fit to view or F while the diagram has focus |

The viewer consumes only selected view identities and rendered metadata. It does
not embed or reparse the input plan. The index explicitly lists identity fields;
attribute values are not added to it or to scripts. HTML also omits Route
destination constants from diagram text, titles and metadata; standalone SVG
retains its existing destination display. Graph selection and routed geometry
remain unchanged.
JSON escapes HTML delimiters, and dynamic UI text uses DOM text nodes rather than
executable markup. CLI-filtered resources do not become searchable. Existing
unavailable-reference descriptions can still identify a target outside the view.
The viewer provides no editing, persistence or additional graph filtering.

## Routing hotspots

Generate with `--format html --html-diagnostics` to include optional final-layout
diagnostics for the effective view. **Show routing hotspots** displays a compact
list and unfilled regions; selecting a finding reveals its region. Expand a path
identity to inspect its state, provenance and representative segment, or navigate
to an available endpoint. Clear/Escape and resource or relationship selection
reset the overlay. The overlay ignores pointer events and leaves paths and
numbered terminals clickable; static SVG output is unchanged.

Independent overlaps and below-minimum border clearance are violations;
crossings, sustained parallel proximity and preferred clearance are advisory.
Regions use the report's whole-diagram coordinates, including named-state offsets
and cross-state gutters. Region dimensions and available minimum visible gaps are
shown without running a second geometry analysis in JavaScript.

The version-1 HTML bridge retains at most 200 groups. Findings with the same kind,
severity and complete path/provenance identities whose origins share a 32px cell
are grouped, with counts, combined bounds and minimum visible gap preserved.
Segments are representative samples within grouped regions. Omitted findings and
incomplete analysis are reported explicitly; a zero count is not proof of perfect
routing. Without the flag, diagnostics are unavailable and the viewer still works.
See [diagnostic approximations and limits](layout-diagnostics.md).

## Conservative impact exploration

Select a resource and use **Explore impact** to list *potentially related
resources* within 1, 2 or 3 hops (default 2). Ordinary one-hop selection remains
the default until that action is used. Resource rows and discovery paths show
change actions, state-qualified identities, recorded relationship direction and
provenance. Unchanged resources can be explored as context; changed resources
retain their existing action colors. Purple dashed outlines mark direct results,
and teal dotted outlines mark additional hops.

Verified dependency arrows run from dependencies to dependents. **Upstream
dependencies** follows incoming arrows; **Downstream dependents** follows outgoing
arrows; **Both** follows either. The optional association checkbox includes only
provenance-backed AWS associations. Directed associations follow their recorded
arrow in the selected direction; undirected associations can be followed in either
direction. These are labeled separately and never asserted to be deployment
dependencies. Inferred connections, containment, unknown semantics, component
membership and unresolved references do not propagate exploration. The panel gives
up to 50 examples of excluded associations or inferred/unknown relationships.

Traversal reads the selected view's structured relationship metadata, including
cross-state relationships; it does not infer relationships from visual proximity,
names, ARNs or attribute values. Equal addresses in different states and deposed
objects retain separate identities. A relationship provenance resource is not
silently converted into an endpoint: its selection details provide endpoint
navigation to start exploration there.

Deterministic breadth-first discovery preserves parallel relationship provenance,
stops revisiting cycles, and shows one discovery path per resource plus a list of
traversed relationships. The graph indexes at most 10,000 relationship records;
each exploration visits at most 200 nodes including its origin, 400 relationships
and 2,000 adjacency entries. Hitting a limit is disclosed. Missing targets are
labeled unavailable and cannot extend a path; CLI-filtered nodes are not restored.
The view cannot prove that an unlisted resource is absent from Terraform.

Search edits, resource/relationship navigation, Clear/Escape and changing exploration
controls reset the impact display. Pan and zoom retain it. All controls work with
the keyboard. This is neither an apply-order prediction nor a runtime blast-radius,
reachability or guaranteed-effect model; the static SVG is unchanged.

Logical-resource type cards show all nonempty action sections with colored borders
and explicit counts, equally in offline SVG and HTML. Policy and Controller have
visible explanations in the card heading. See the synthetic
[all-action example](../examples/logical-actions.svg).

The interactive per-type action filter from #373 is an explicit follow-up. It
requires coordinated annotation/state/canvas reflow and selection/reference target
availability, including visible counts and keyboard-accessible controls. The current
viewer has no runtime layout engine: hiding rows alone would leave stale geometry
and links. That follow-up must distinguish filtered targets from absent resources
and must never restore resources excluded by CLI focus or changed-only selection.

The Relationship lines card has a 960px width cap in both SVG and HTML. Its five
entries use two columns (2 / 2 / 1), falling back to one when space is narrow.
The architecture canvas retains its own width; named states share one legend.

The [representative example](../examples/viewer.html) includes logical resources,
Uses references, unresolved references, managed/data resources and component
membership. Regenerate it from the committed fixture:

```sh
docker compose exec -T dev cargo run --locked -- tests/fixtures/viewer-plan.json --format html -o examples/viewer.html
```

HTML is maintained with this command, separately from the SVG writeback manifest;
the representative HTML snapshot is checked by the CLI integration tests.
Rust CLI tests check safe serialization, target IDs, selection and unchanged SVG
content. `docker compose exec -T dev python3 tests/browser/viewer.py` runs Chromium
against local files with networking disabled, checking search, keyboard controls,
navigation, references, membership and preservation of edge geometry and styles.
Build the CLI with `cargo build --locked` in the same container first.

## Viewer source modules

`src/html/viewer.js` initializes the viewer and coordinates selection/reset.
`metadata.js` owns identity and DOM indexes; `selection.js` renders details and
relationship navigation; `searching.js` owns search/results; `navigation.js` owns
the view box and pan/zoom; `highlighting.js` owns selection highlighting cleanup.
`lifecycle.js` registers optional feature adapters with a `reset()` method. New
overlays register there so resource/relationship selection and Clear cannot leave
stale feature state. Modules receive their dependencies explicitly and return
small interfaces; their functions remain inside the viewer's private closure.
`diagnostics.rs` reduces the final report to the bounded HTML bridge;
`diagnostics.js` consumes it and registers its reset adapter.
`impact-graph.js` indexes verified graph metadata and performs bounded traversal;
`impact.js` owns exploration controls, summaries and transient highlighting.

`src/html.rs` concatenates the sources in a fixed order with `include_str!` and
`concat!` at compile time, inside one inline script. Edit the source modules and
rebuild Rust; no JavaScript package installation, bundler or runtime imports are
needed. Keep initialization in the coordinator and add new sources to that list.
Run `docker compose exec -T dev node --test tests/browser/modules.cjs` for module
contracts, plus the browser and Rust CLI tests above. Regenerate both maintained
HTML files after source changes; use `scripts/render-multi-account.sh` for the
combined example. HTML snapshot comparisons verify deterministic bundling.
