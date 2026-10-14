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
