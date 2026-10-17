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
changed-only and module selection, remote-state mappings and diagnostics work in both modes.
`--hide-relationship-legend` applies during SVG generation in both formats;
there is no viewer toggle. It keeps action colors and relationship navigation.
Logical Resources rows show the same per-resource deployment-scope labels in both
formats; see [scope evidence and limitations](aws.md#deployment-scopes).

Use `--coverage-report PATH` for a separate Markdown report of input/selected/
excluded resources and unknown scope counts. SVG and HTML share the same
[coverage accounting](coverage.md); the diagram reserves no summary space.

The maintained [combined AWS viewer](../examples/multi-account/diagram.html)
exercises 250 input instances, repeated addresses in three states, region
backgrounds and remote logical/spatial references. Its [generation and review
commands](../examples/multi-account/README.md) use the same inputs as its SVG.

Search matches the complete native and qualified address, architecture identity,
module, state, resource type, provider source, managed/data mode and deposed key. Space-separated
terms must all match. Results retain full addresses and state/mode distinctions.
Action, State and Resource type / provider source each offer a single optional
filter. All filters and text terms combine with AND; defaults include everything
in the generated searchable index. Actions are Create, Update, Delete, Replace,
Read, Unchanged and Other; import, moved and drift are separate metadata, not
actions. Unknown provider sources are labeled explicitly. Search results have no
display-count limit: the count reports all matches against the generated index.
Filters only narrow the result list, leaving diagram geometry, coverage counts
and existing selection intact. A missing result does not mean a resource was
removed from Terraform or the diagram. **Reset search and filters** clears all
text and facets and focuses search; Clear selection/Escape retains them. Facet
changes reset transient impact and diagnostic overlays like text search does.
Selecting a result or diagram resource reveals its individual stable target,
including nested logical-resource rows, embedded Route cards and RDS cluster members.
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
| Search | Type text and optionally select action/state/type filters; Enter in search selects the first result |
| Reset search | Reset search and filters clears text and all three facets |
| Results | Arrow Down enters results; Up/Down and Home/End navigate; Enter selects |
| Resource links | Tab to a resource or link, then Enter; clicking also selects |
| Relationship numbers | Click, or Tab then Enter/Space; endpoint buttons navigate to resources |
| Copy address | Copy Terraform address in selection details; Tab then Enter also works |
| Selection history | Back/Forward buttons; Tab then Enter also works |
| Clear selection | Escape or Clear selection |
| Pan | Drag or scroll; arrow keys while the diagram has focus |
| Zoom | +/− buttons, +/− keys in the diagram, or Ctrl/Command+scroll |
| Fit entire view | Fit to view or F while the diagram has focus |
| Export visible area | Export viewport SVG; Tab then Enter also works |

**Export viewport SVG** downloads a standalone `planorama-viewport.svg` using the
current viewBox. The file intentionally crops content outside that viewport;
it retains the generated layout, nested state offsets, definitions, icons, action
colors and font fallback lists. Font availability can vary between computers.
Selection, impact and routing diagnostic overlays are omitted from the clone;
the live selection and viewport remain unchanged. Internal links remain active
only when their destination exists in the exported document. Unavailable links
become inert with an explanatory title. No HTML index, JavaScript, original plan
or additional attribute values are included. Download failures are announced beside
the control; if the browser silently blocks saving, check its download permissions.

Select a resource and choose **Export selected neighborhood SVG** for an extract.
Depth is 0–3 (default 1). Traversal follows generated relationships in either
direction, including directed/undirected associations, inferred connections and
other recorded line kinds. It preserves their original directions, styles and
structured provenance; this is not runtime impact exploration. A provenance
resource also seeds its represented endpoints. Containment does not consume hops:
required ancestors, embedded owners, relationship sources and complete numbered
entries are added as context. These can extend beyond the requested depth.

Only resources in the effective generated view are eligible. Missing neighbors
are counted but never recreated. Traversal examines at most 10,000 indexed
relationships and 2,000 adjacency entries, retaining at most 200 resources and
400 relationships. Truncation is explicit. If required context or faithful
presentation exceeds 200 resources, 400 paths/entries or 100,000 SVG elements,
export is cancelled visibly rather than saving an incomplete representation.

The extract preserves resource coordinates, path geometry, nested state offsets,
clipping and retained annotation frames. It removes unrelated resource rows,
cards, paths and mask occluders, without repacking. Empty space can remain inside
retained frames; the crop encloses retained content with 24px padding and a scope
caption. The caption and SVG metadata report unique retained resources, additional
context/provenance and rendered path counts. Logical action counts reflect retained
rows. Original whole-plan counts are omitted. Links to removed destinations become
inert explanations. The live diagram, selection and viewport remain unchanged.

Back/Forward restores earlier selections and their viewport positions. History
holds up to 100 selections in memory, distinguishing state-qualified resources,
current/deposed instances, relationship entries and provenance resources. Leaving
a selection saves its latest pan/zoom position; pan/zoom alone and repeated
selection of the same target do not add entries. A new selection after Back
discards the forward branch. Missing targets are skipped safely and never rebuilt.
Restoration resets impact and diagnostic overlays through normal selection and
focuses the restored target. Clear selection/Escape empties history; search edits,
facets, Reset search and Fit retain it. Reloading the file starts fresh. These
controls use no browser-history shortcuts, storage, URL state or network access.

Selection details offer the native Terraform address, independent of address-format
settings, with managed/data, current/deposed and state shown separately. Relationship
details distinguish verified provenance resources from endpoint resources. Copy runs
only on request and prefers the secure-context Clipboard API. If unavailable or
denied, the address is selected for manual Ctrl+C/Command+C copying; success is
announced only after the clipboard write completes. No attributes or plan body are
copied, and no clipboard reading, network access or storage is used.

The viewer consumes only selected view identities and rendered metadata. It does
not embed or reparse the input plan. The index explicitly lists identity fields;
attribute values are not added to it or to scripts. HTML also omits Route
destination constants from diagram text, titles and metadata; standalone SVG
retains its existing destination display. Graph selection and routed geometry
remain unchanged.
JSON escapes HTML delimiters, and dynamic UI text uses DOM text nodes rather than
executable markup. CLI-filtered resources do not become searchable. Existing
unavailable-reference descriptions can still identify a target outside the view.
The viewer provides no editing or saved viewer-state persistence. Neighborhood
export filters a copy; the live diagram remains unchanged.

## Routing hotspots

Routing hotspots describe final geometry. They are independent of the semantic
findings described below and can be enabled alongside them.

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

## Semantic diagnostics

Generate with `--format html --html-semantic-diagnostics` to embed selected-view
semantic findings. Without the flag, no semantic record payload is included.
`--diagnostics` continues to write its existing stderr output independently.

```sh
planorama plan.json --format html --html-semantic-diagnostics -o diagnostics.html
planorama --state app=app.json --state network=network.json --format html --html-semantic-diagnostics -o diagnostics.html
```

**Show semantic diagnostics** opens an offline search and reason-category filter.
Rows show the full state, address, managed/data mode, current/deposed identity,
stable reason code, explanation and attribute path when available. Buttons select
existing spatial cards, logical rows, embedded Routes or lowered relationship
provenance. Missing navigation targets remain explanatory text. Selection never
restores CLI-excluded resources or fabricates semantic relationships.

Embedding retains at most 1,000 complete records within a 256 KiB escaped JSON
budget, including envelope space. Oversized records are omitted whole, preserving
full identifiers on retained records. Ordering is state, source identity,
attribute and reason. The panel renders at most 100 matching embedded records;
search/filter can narrow the result. Embedded omissions and list-limit omissions
are reported separately. Counts refer to findings, not unique affected resources.

Analysis covers supported state-local rules only; cross-state mapping diagnostics
remain on stderr. Input completeness, unavailable collection, zero findings,
unassociated input findings and truncation remain distinct. Unknown scope is
informational. Only safe identifiers and explanations are embedded, never values,
provider settings, raw expressions or the plan. See [coverage accounting](coverage.md).

Controls support Tab, Enter and native select/search keyboard operation. Selecting
a finding focuses its resource; Clear selection, Escape and resource search close
the panel through the shared viewer lifecycle. Escaping the panel returns focus
to its toggle. Routing hotspots and semantic diagnostics use separate panels.

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

Search facets filter results, not the diagram. Runtime diagram filtering and
reflow remain unimplemented; CLI [selection](filtering.md) determines the generated
view. The viewer cannot restore excluded resources.

SVG and HTML share the [relationship legend](relationships.md#legend-and-meaning),
including conditional Subscription, Event source and Event target entries and
the compact `--hide-relationship-legend` presentation.

The [representative example](../examples/viewer.html) includes logical resources,
Uses references, unresolved references, managed/data resources and component
membership. Follow [HTML regeneration](../CONTRIBUTING.md#html-の更新), separately
from the SVG writeback manifest. CLI integration tests check this HTML snapshot.
Rust CLI tests check safe serialization, target IDs, selection and unchanged SVG
content. `docker compose exec -T dev node tests/browser/viewer.cjs` runs Chromium
against local files with networking disabled, checking search, keyboard controls,
navigation, references, membership and preservation of edge geometry and styles.
The Node-only WebDriver helper uses ChromeDriver's CDP commands and performance
logs to reject attempted external HTTP(S) requests even while offline, and fails
on severe browser console errors. Conditions and protocol requests have bounded
timeouts; browser, driver and temporary files are cleaned up on failure.
Run `docker compose exec -T dev node --test tests/browser/webdriver.test.cjs`
to verify the monitor's failure paths and cleanup.
Build the CLI with `cargo build --locked` in the same container first.

## Viewer source modules

`src/html/viewer.js` initializes the viewer and coordinates selection/reset.
`metadata.js` owns identity and DOM indexes; `selection.js` renders details and
relationship navigation; `searching.js` owns search/results; `navigation.js` owns
the view box and pan/zoom; `highlighting.js` owns selection highlighting cleanup.
`selection-history.js` stores bounded selection/viewport history and restores it
through selection and navigation, preserving the shared overlay reset lifecycle.
`export.js` clones, cleans and validates standalone SVG downloads without changing
the live viewer.
`neighborhood.js` performs bounded traversal using the structured graph index.
`export-presentation.js` maps retained identities to resource cards, relationship
entries, containment/scope frames, annotations and masks. The generated HTML index
provides containment/ownership and scope context; SVG occluders and junctions carry
explicit resource memberships. Projection preserves geometry and validates identities
before invoking shared standalone serialization.
`lifecycle.js` registers optional feature adapters with a `reset()` method. New
overlays register there so resource/relationship selection and Clear cannot leave
stale feature state. Modules receive their dependencies explicitly and return
small interfaces; their functions remain inside the viewer's private closure.
`diagnostics.rs` reduces the final report to the bounded HTML bridge;
`diagnostics.js` consumes it and registers its reset adapter.
`semantic_diagnostics.rs` bounds selected semantic records and escaped bytes;
`semantic-diagnostics.js` handles their search, categories and source navigation
through the same reset lifecycle.
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
