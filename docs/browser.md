# Browser generation

The browser API accepts a `terraform show -json <saved-plan>` JSON string or explicit named inputs and
returns SVG or self-contained HTML in memory. It uses the native CLI's parser,
semantic rules, selection, layout and rendering. JSON-only AWS accounts remain
unknown; no provider evidence or account inference is added.

Generated HTML includes the shared [generation information contract](viewer.md#generation-information),
including effective selection settings, generator version and the revision compiled
into the Wasm engine. It never substitutes the hosting site's revision. Setting
origins remain unavailable. [Build provenance](../CONTRIBUTING.md#build-provenance)
uses the same acquisition and fallback rules for native and Wasm builds.

## 読者別の入口

- 利用者: [起動](browser-development.md#build-and-run)、[設定](#sample-ui-settings)、[named inputs](#named-state-inputs)、[privacy](#privacy-and-lifecycle)、[制限](#limits-and-verification)。
- API 利用者: [共通 API・options](#api-and-supported-options)、[remote-state mapping](#explicit-remote-state-mappings)、[engine・分割実行・memory](browser-engine.md)。
- 開発者: [build・検証](browser-development.md)、[CI 運用](ci.md#browser-wasm-432)。生成後の HTML 操作は [viewer](viewer.md) が正本です。

## Build and run

[Build and run](browser-development.md#build-and-run) を参照してください。

## Standalone engine without artwork

[Standalone engine without artwork](browser-engine.md#standalone-engine-without-artwork) を参照してください。

## Resumable generation

[Resumable generation](browser-engine.md#resumable-generation) を参照してください。

## API and supported options

Native Rust callers use `planorama::generate(json, &planorama::Options::default())`
and receive `Result<Generated, Error>`. `Generated` contains `content` and
`diagnostics`. Internal parser, graph and layout types are not public API.
The CLI retains its existing options and detailed diagnostics; the public API
uses typed errors and deduplicated static diagnostic descriptions without
addresses, paths, input excerpts or attribute values.

The generated JavaScript binding exposes
`wasm_bindgen.generate(jsonString, optionsJsonString)`. After `initSync`, it
returns a JSON string with `content` and `diagnostics`. Errors throw a JSON string
with `code` and `message`. Codes are `input_too_large`, `invalid_json`,
`unsupported_input`, `invalid_options`, `selection_failed`, `generation_failed`.
Runtime traps are handled by the Worker as a generic generation failure.

`planorama::inspect_modules(json)` returns `Result<Vec<ModuleInventoryEntry>, Error>`;
`wasm_bindgen.inspect_modules(jsonString)` returns the same entries as a JSON array.
Both use the CLI inventory collector on the original parsed input, without semantic
projection, selection, layout or routing. Each entry has `selector` (null for root),
`instance` (false for configuration aggregates), `direct` and `subtree` counts.
Counts include distinct managed/data instances and each current/deposed object;
subtree includes direct resources. Overlapping rows must not be summed. Empty
instances and configuration modules with no known instances have zero counts;
configuration evidence does not prove an indexed instance exists. Inspection uses
the same input limit and safe error contract as generation.

`planorama::inspect_resources(json)` returns `Result<Vec<ResourceInventoryEntry>, Error>`;
the Wasm export returns these rows as JSON. `inspect_input(json)` returns both
`modules` and `resources` using one parse, and is used by the sample Worker.
Neither API performs semantic projection, selection, layout or routing. Resource
rows contain only `state`, `selector`, original `address`, `resource_type`, `module`,
`mode` (`managed`/`data`), and `current`/`deposed` object counts. One row groups all
objects sharing an exact focus address; it does not offer individual historical
objects. Generation still rejects ambiguous focus according to shared selection
rules. No attributes or deposed keys are returned. Single-plan `state` is null;
named-state inventory groups by state and address and returns
`STATE:ADDRESS` selectors. Consumers must pass `selector` unchanged.

| Option | Values / default |
| --- | --- |
| `format` | `svg` (default), `html` |
| `address_format` | `qualified` (default), `terraform` |
| `icon_mode` | `service-name` (default and only supported mode; renders resource/service labels) |
| `hide_relationship_legend` | Boolean, false |
| `omit_relationship_legend` | Boolean, false; omits the entire card, overriding `hide_relationship_legend` |
| `html_semantic_diagnostics` | Boolean, false; requires `format: "html"` |
| `html_diagnostics` | Boolean, false; requires `format: "html"` |
| `changed_only` | Boolean, false |
| `focus` | Resource address or null |
| `focus_depth` | Nonnegative integer; requires focus; default neighbor depth 1 |
| `modules` | Array of module addresses, default empty |
| `spatial_types` | Array of the seven [allowed AWS types](configuration.md#explicit-spatial-types), default empty |
| `remote_state_mappings` | Array of explicit named-state mappings, default empty; [schema below](#explicit-remote-state-mappings) |

Unknown keys, invalid enum values and invalid combinations fail explicitly.
### Sample UI settings

Both diagnostic options reject SVG with `invalid_options`; the form explains
the conflict and focuses the enabled diagnostic checkbox. Switching formats
retains checkbox values. Reset clears both.
The sample UI exposes the options above except remote-state mappings, which are
available through the Rust/Wasm API. Generation settings remain
visible and use the existing API decoder and selection rules.

**Semantic findings** and **Routing hotspots** independently enable the existing
[HTML viewer panels](viewer.md#routing-hotspots). Rust `Options` and Wasm JSON
use the names above; CLI equivalents are `--html-semantic-diagnostics` and
`--html-diagnostics`. The default output embeds neither diagnostic payload.
Semantic findings describe supported rules for the selected resources; geometry
findings describe final layout and routes. Navigation uses the selected view's
resource index. Unavailable targets remain explanatory text.

These reuse the native collectors and bounded bridges: semantic records are
limited to 1,000 and 256 KiB, routing groups to 200. Panels distinguish
uncollected diagnostics from collected zero findings, report omitted entries and
incomplete analysis/input (including unknown input completeness). Cross-state
mapping diagnostics are returned in `Generated.diagnostics`, not these panels.
Provider evidence is unsupported; this is not complete analysis. Only existing
identifiers and fixed explanations are embedded, without
raw attribute values, expressions, provider settings or the plan body. Diagnostic
settings share snapshot, reset, cancellation and offline sandbox preview behavior.

Choose the output and address format, and optionally hide relationship
explanations (retaining the action color key), omit the entire relationship legend,
or enable changed-only. Both legend settings default to false; entire omission
takes precedence. They apply at generation time to SVG and HTML and participate
in the settings snapshot, reset and cancellation of stale results. For example,
`generate(json, JSON.stringify({omit_relationship_legend: true}))` removes the
legend and its reserved space without changing relationships or resource cards.
Enter a focus resource address to
enable depth: empty uses the API default of 1, and 0 selects no neighbors.
Explicit depth must be decimal digits in the Wasm nonnegative integer range
0–4294967295; invalid input is announced and focused without correction.
Clearing focus disables depth and omits it from requests, retaining the field
value for when focus is entered again.

Choosing a file automatically lists resources and modules. The focus picker searches
address, type, module, mode and state, showing total matches and a range of up to
50 addresses. Tab/Space selects one address and synchronizes the manual focus field;
selecting another replaces it. Uncheck or **Clear focus** to clear it. Search and
pagination retain the exact selected string, including indexed-key whitespace,
quotes, commas and backslashes. Manual selectors absent from the list are retained
for generation validation. Presence in the original input does not guarantee
rendering after modules, changed-only, depth and context closure are applied.

Search the original module inventory and
use **Previous modules** / **Next modules** to browse 50 rows at a time. Checkboxes
support Tab and Space; root is disabled. Selection and the selected-selector count
survive searching and pagination. Picking an entry adds a selector row; unchecking
it removes every matching row. Manual edits update the corresponding checkbox.
The rows are passed to the existing `modules` option, sharing CLI focus,
changed-only and context-closure rules. Generated HTML runtime filters are unchanged.

Use **Add module** to enter one complete selector per row. Rows preserve spaces,
commas, dots, quotes and escapes inside instance keys; they are not split or
trimmed. No rows sends an empty array (no module filter). Empty rows must be
filled or removed. Duplicate rows are passed unchanged to the API, which selects
each resource once. Removing a row moves keyboard focus to an adjacent selector
or the Add module button. Invalid selectors use the API's safe selection error.

Use **Spatial resource types** in Generation settings to select any combination
of the seven AWS types. No selection preserves the default Logical presentation.
Selections pass exact type names to the Wasm API; reference details and resource
identity follow the shared native/library projection rules. The build generates
labelled checkboxes from `src/view/purpose.rs`'s `TYPES` definition using
`scripts/build-spatial-options.cjs`, so the UI and decoder share the allowed list.
Each checkbox supports keyboard focus and Space to toggle.

**Reset settings** restores every option, clears spatial selections, removes module rows and clears depth,
while retaining the selected file and completed inventories. It also clears focus
and both searches, cancels pending work and clears results. Replacing the file
clears focus, module selectors, both searches and old inventories before reading;
other settings (including the depth field) are retained. Either reload-list button
retries both inventories after cancellation or failure without clearing selectors.
Cancellation and read/Worker failures retain completed lists; reported input
errors clear the lists. Both preserve manual selectors. Changing generation settings
cancels pending inspection; search and pagination do not. Stale reads and Worker
completions cannot replace a newer list. Completed inventory remains unfiltered.
State JSON, plan event streams, saved binary plans, Terraform
execution, provider evidence, config/preset files, coverage/layout
report files are unsupported in this API.

## Named state inputs

Enable **Use named states**, then **Add state input** for each file. Enter each
state name explicitly; filenames never supply state identity or AWS accounts.
Names follow the [native multi-plan contract](multi-plan.md): case-sensitive,
unique, 1–64 ASCII letters, digits, periods, underscores or hyphens. A single
named input also keeps its state-qualified identity. Disable named mode to return
to the existing unnamed file input; files in the inactive mode are excluded.

Rust exposes `NamedInput { state: String, json: String }`,
`generate_named(&[NamedInput], &Options)` and `inspect_named_inputs(&[NamedInput])`.
Wasm exposes `generate_named(inputsJson, optionsJson)`,
`generate_named_with_progress(inputsJson, optionsJson, callback)` and
`inspect_named_inputs(inputsJson)`. `inputsJson` is a JSON array of objects with
exactly `state` and `json` string fields. These are synchronous APIs intended for
a fresh Worker. For bounded cooperative execution, use
`Generation.named(inputsJson, optionsJson)` as described under
[resumable generation](browser-engine.md#resumable-generation); its aggregate limits are smaller.

```js
const inputs = [{state: 'network', json: networkJson},
                {state: 'application', json: applicationJson}];
const result = JSON.parse(wasm_bindgen.generate_named(JSON.stringify(inputs),
  JSON.stringify({format: 'svg', modules: ['application:module.app']})));
```

All names and the combined input size are validated before parsing any plan.
For synchronous generation and inspection, there must be 1–64 inputs; each file and the sum of decoded JSON UTF-8 bytes are
limited to 32 MiB. The UI checks file sizes before reading. Named-input envelope
bytes have an additional Wasm pre-deserialization bound of 192 MiB + 16 KiB to
allow JSON escaping; this does not increase the decoded input allowance.
`invalid_state_name`, `duplicate_state_name` and `empty_inputs` supplement the
existing safe error codes. The UI identifies missing/invalid/duplicate input rows
and failed file reads by position without exposing file contents or runtime errors.

Inspection includes a `state` on each module and resource row. Module roots stay
unselectable and show per-state direct/subtree counts; other module and focus
selectors are `STATE:ADDRESS`. Identical addresses and current/deposed objects in
different states stay independent. Counts refer to original inputs, before filters.
Generation shares native state-local parsing, semantic transformation, selection
and SVG/HTML rendering. Explicit remote-state mappings use the shared CLI resolver
as described below. Provider evidence remains CLI-only. Equal names or ARNs do
not automatically create cross-state connections.

Each request snapshots all active files, names and generation settings before
reading. Adding, removing or replacing a file, renaming a state, or switching input
mode cancels the Worker, invalidates pending reads and results, clears inventories,
focus, module selectors and searches, and retains other settings. Reload either
list once inputs are ready. Settings edits cancel work and results while retaining
completed inventories and selections. Reset retains all chosen files and state
names. Cancel/read failure retains manual selectors for retry; stale callbacks
cannot publish an inventory or result. No partial input set is generated.

`tests/fixtures/browser-named.json` deliberately reuses the same addresses across
two named states. Browser checks compare exact native/Wasm SVG and HTML, module
and focus selection, data/deposed counts, safe validation, read failure,
cancellation/retry and the aggregate byte limit, with networking disabled.
`tests/browser/named-resumable.cjs`, invoked by the memory suite for both Wasm
distributions, checks sliced SVG/HTML equality, selection, stage observation,
invalid units, cancellation/retry, safe closure and aggregate refusal. The memory
suite also exercises named jobs under every supported memory maximum, allocation
failure, trap isolation and fresh-instance retry.

### Explicit remote-state mappings

Rust `Options.remote_state_mappings` is a `Vec<RemoteStateMapping>`, where the
public type has three required `String` fields: `consumer`, `remote`, `producer`.
The same option works with synchronous/progress generation and resumable named
jobs in Rust and both Wasm distributions. The JSON schema is:

```json
{
  "remote_state_mappings": [
    {
      "consumer": "application",
      "remote": "module.app.data.terraform_remote_state.network",
      "producer": "network"
    }
  ]
}
```

`consumer` and `producer` are exact loaded state names; `remote` is the exact
consumer-local data-source address, without a state prefix or `.outputs` suffix.
Indexed module/resource selectors are preserved verbatim. This is equivalent to
CLI `--remote-state application:module.app.data.terraform_remote_state.network=network`.
No mapping is inferred from names, filenames, literal IDs, ARNs or backend values.
An omitted/empty array preserves existing callers; nonempty mappings on a
single unnamed input return `invalid_options`. Inspection does not take mappings.

Validation runs against original inputs before filtering. Unknown fields, missing
fields, nulls and non-string fields return `invalid_options` during JSON decoding.
Invalid names, empty/control-bearing addresses, missing or non-unique current
data sources, wrong types/modes and non-Terraform providers return
`invalid_remote_state_mapping`. Missing consumer/producer states return
`unknown_remote_state_mapping`. Conflicting producers for the same consumer and
remote address return `ambiguous_remote_state_mapping`; identical duplicates
are idempotent. Errors contain fixed explanations and never echo supplied values.

Valid mappings reuse native output-provenance resolution, state-qualified identity,
provider checks, current/deposed handling and selection/context closure. Missing
or incomplete producer outputs emit no guessed edges. When mappings are supplied,
`Generated.diagnostics` includes the resolver's deduplicated fixed failure
descriptions, including unmapped references; it excludes expressions, output
values, credentials and input excerpts. Filtered targets follow native selection
rules; mappings never bypass filters. SVG/HTML bytes match the CLI for identical
inputs, mappings and options.

There are at most 256 mappings, each remote address is at most 4,096 UTF-8 bytes,
and all mapping fields share the 64 KiB decoded option-string allowance with
module, spatial-type and focus selections. Every Wasm generation entry point
also bounds the serialized options envelope to 64 KiB before JSON decoding.
Limit violations return `resource_limit`; input/envelope and aggregate resumable
budgets remain independent. The host must bound strings before transferring them
into Wasm, because binding argument copies precede Rust validation.

Rust tests cover resolution, validation, safe diagnostics, special characters,
repeated addresses, historical objects, filtering and aggregate cross-state
fanout. Browser tests compare mapped SVG/HTML with the native CLI; memory tests
cover mapped synchronous/resumable parity, cancellation and typed failures.

## Privacy and lifecycle

During generation the UI shows parsing, interpretation/selection, placement,
routing and output composition at their actual pipeline boundaries. Routing
includes geometry refinement and may remain visible for a long time. These
stages are not percentages or predictions of remaining time; only a successful
result indicates completion. Elapsed seconds use the page's monotonic clock,
starting before file reading, and stop on completion, failure or cancellation.
Only stage changes are announced to screen readers. Cancel terminates the Worker;
settings changes, file replacement and retry invalidate its notifications and timer.
Module and resource inspection share a status and do not report generation progress.

The optional synchronous Wasm `generate_with_progress(json, options, callback)`
uses the same pipeline and limits as `generate`. The callback receives only a
fixed stage name (`parsing`, `selection`, `placement`, `routing`, `output`);
callback exceptions do not change generation. The sample Worker suppresses
repeated stages and associates notifications with the current request. Neither
progress nor elapsed time contains plan identifiers or attributes or is persisted.

Only static assets are fetched at startup, without credentials. After the Ready
message the page holds the compiled Wasm module and Worker source in memory.
Module inspection, generation, preview and download work with the network disabled while this page
remains open. Reloading requires fetching the assets again; there is no service
worker or persistent offline cache managed by the sample.

Input, output and diagnostics are never sent over the network or automatically
written to localStorage, sessionStorage or IndexedDB. Downloads are explicit.
Each request uses a new Worker; completion, cancellation and failure terminate
it. Generate snapshots all settings before reading the file. Editing any setting,
changing the file, resetting or cancelling terminates pending generation and
invalidates pending file reads and stale completion callbacks. Edits stay visible;
generate again to apply them. Failure leaves settings available for correction
and retry. Blob URLs from prior results
are revoked. Retrying after failure or cancellation does not reload remote assets.

Preview uses an opaque-origin sandboxed iframe with scripts allowed for the HTML
viewer, but no same-origin access, forms, popups or top navigation. A preview-only
CSP blocks connections and external content; original downloaded content is
unchanged. Input strings use the renderer's existing escaping, and status text
uses `textContent`. The host page never inserts a generated diagram into its DOM.
The host CSP permits inline scripts because Blob previews inherit its policy and
the self-contained viewer contains inline JavaScript; the preview's own CSP and
opaque-origin sandbox still restrict that script's capabilities.

Plan JSON may contain secrets. The generated diagram still contains resource
identifiers, and SVG may show validated route destination constants; HTML omits
those constants. See [route display](routes.md) and [provider evidence privacy](provider-evidence.md#privacy-and-validation).
Keeping generation local does not make the explicit download anonymous.

## Limits and verification

The library and file picker limit UTF-8 input to 32 MiB. This is an input limit,
not a memory or duration guarantee: JSON trees, graph expansion, routing search,
output strings, structured-clone copies and the preview consume additional memory.
Very dense or oversized plans may exhaust a browser process or Wasm memory before
a recoverable error is possible. Use Cancel for long work and reduce the plan or
use CLI selection for inputs that exceed practical browser capacity. Worker
termination allows the runtime to reclaim its Wasm allocation, without promising
immediate OS reclamation; a fresh request does not retain its
high-water memory. Worker measurements report Wasm linear memory size, not total
browser RSS or JavaScript heap usage.

検証コマンド・coverage・計測条件は [開発者向け検証](browser-development.md#verification) を参照してください。

## Per-generation memory selection

[初期化・上限・失敗時の lifecycle](browser-engine.md#per-generation-memory-selection) を参照してください。
