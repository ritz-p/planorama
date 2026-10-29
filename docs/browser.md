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

| API | Result |
| --- | --- |
| Rust `planorama::generate(json, &planorama::Options::default())` | `Result<Generated, Error>`; `Generated` has `content` and `diagnostics` |
| Wasm `wasm_bindgen.generate(jsonString, optionsJsonString)` after `initSync` | JSON string with the same fields; errors throw a JSON string with `code` and `message` |
| Rust `planorama::inspect_modules(json)` | `Result<Vec<ModuleInventoryEntry>, Error>` |
| Rust `planorama::inspect_resources(json)` | `Result<Vec<ResourceInventoryEntry>, Error>` |
| `inspect_input(json)` | Both inventories from one parse; used by the sample Worker |

Wasm `wasm_bindgen.inspect_modules(jsonString)` and
`wasm_bindgen.inspect_resources(jsonString)` serialize the corresponding arrays;
`inspect_input(json)` serializes both in an object. Inspection
uses the original input and CLI collectors without projection, selection, layout
or routing; it shares generation's input limit and safe errors. Parser, graph and
layout types are private. Public diagnostics are deduplicated static descriptions
without addresses, paths, excerpts or values; CLI diagnostics remain detailed.
Generation codes are `input_too_large`, `invalid_json`, `unsupported_input`,
`invalid_options`, `selection_failed`, `generation_failed`; the Worker reports
runtime traps as generic generation failures.

Module rows contain `selector` (null for root), `instance` (false for
configuration aggregates), `direct` and `subtree` counts. Counts distinguish
managed/data and current/deposed objects; subtree includes direct resources,
so overlapping rows must not be summed. Empty/configuration-only modules count
zero and do not prove indexed instances exist.

Resource rows contain only `state`, `selector`, original `address`,
`resource_type`, `module`, `mode` (`managed`/`data`) and `current`/`deposed`
counts. Each exact focus address groups its objects; historical objects are not
individually selectable and ambiguous generation focus still fails. Attributes
and deposed keys are excluded. Single-plan `state` is null; named inventories
group by state/address and return `STATE:ADDRESS`. Pass selectors unchanged.

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

The sample UI exposes all options except remote-state mappings. Diagnostic
checkboxes require HTML; SVG conflicts return `invalid_options`, focus the enabled
checkbox and explain the error. Format changes retain checkbox values; Reset clears them.

**Semantic findings** and **Routing hotspots** enable the independent
[viewer panels](viewer.md#routing-hotspots), equivalent to
`--html-semantic-diagnostics` and `--html-diagnostics`. Both default off.
Semantic records describe selected-resource rules (1,000 records / 256 KiB);
routing groups describe final geometry (200 groups). Panels distinguish
uncollected, zero, omitted and incomplete/unknown-input results, use the selected
resource index and show unavailable targets as text. Cross-state diagnostics go
to `Generated.diagnostics`. Only identifiers/fixed explanations are embedded,
not values, expressions, provider settings or the plan body. Provider evidence
remains unsupported; diagnostics share the normal settings lifecycle and sandbox.

Legend options apply to SVG/HTML at generation time: hiding explanations retains
the action key, while `omit_relationship_legend: true` removes the whole legend
and reserved space without changing resources/relationships. Both default false;
omission takes precedence. Changed-only and address/output format use the API rules.
Focus enables depth: blank defaults to 1, 0 selects no neighbors; explicit input
must be decimal digits in 0–4294967295. Invalid depth is announced/focused without
correction. Clearing focus disables/omits depth but retains its field value.

Choosing a file loads unfiltered module/resource inventories. Both pickers show
50 rows per page, retain selections through search/pagination and support Tab/Space.
The focus search matches address, type, module, mode and state. It shows total
matches/range; selecting one address replaces and synchronizes manual focus.
Uncheck or **Clear focus** clears it. Selectors preserve indexed whitespace,
quotes, commas and backslashes. Manual entries absent from inventory remain for
generation validation; inventory presence does not bypass filters/context closure.

Module roots are disabled. **Previous modules** / **Next modules** page through
the inventory; selected counts persist. Checking adds a selector row, unchecking
removes all matching rows, and manual edits update checkboxes. **Add module** takes
one complete, untrimmed/unsplit selector per row, including dots and escapes.
No rows means an empty `modules` array; empty rows must be filled or removed.
Duplicates reach the API unchanged but select each resource once. Removal focuses
an adjacent row or Add module. Invalid selectors use the API's safe error;
CLI focus/changed-only/context closure apply, without changing HTML runtime filters.

**Spatial resource types** passes exact names for any subset of the seven allowed
AWS types; none preserves Logical defaults and native identity/reference rules.
`scripts/build-spatial-options.cjs` generates labelled Tab/Space checkboxes from
`src/view/purpose.rs`'s `TYPES`, keeping UI and decoder lists aligned.

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

Named requests follow the [shared lifecycle](#privacy-and-lifecycle), snapshotting
active files, names and settings before reading. Adding/removing/replacing files,
renaming states or switching mode clears inventories, focus, module selectors and
searches while retaining other settings; reload either list when ready. Reset
retains chosen files/names. No partial input set is generated.

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

The UI reports real `parsing`, `selection`, `placement`, `routing` and `output`
boundaries; routing includes refinement. Stages are neither percentages nor time
estimates, and only a successful result means completion. Monotonic elapsed time
starts before file reads and stops on completion/failure/cancel. Screen readers
announce only stage changes; inspection shares a status without generation progress.

`generate_with_progress(json, options, callback)` shares synchronous `generate`
limits/pipeline and sends only those fixed names; callback exceptions do not alter
generation. The sample Worker suppresses repeats and binds notifications to the
current request. Progress/time contains no plan data and is not persisted.

Startup fetches static assets without credentials. After Ready, the compiled
module and Worker source stay in memory: inspection, generation, preview, download
and retry work offline while the page stays open. Reload fetches assets again;
the sample has no service worker or persistent offline cache.

Input/output/diagnostics are never uploaded or automatically stored in localStorage,
sessionStorage or IndexedDB; downloads are explicit. Each request snapshots settings
before reading and uses a fresh Worker, terminated on completion, failure or cancel.
Settings/file edits, Reset, Cancel and retry invalidate pending reads, callbacks,
notifications and timer; prior Blob URLs are revoked. Edits remain visible for
the next Generate, and failures retain settings for correction. Inventory retention
and selector reset rules are in [UI settings](#sample-ui-settings).

Preview is an opaque-origin sandboxed iframe allowing viewer scripts but no
same-origin access, forms, popups or top navigation. Its CSP blocks connections
and external content without changing downloads. The renderer escapes inputs,
status uses `textContent`, and generated diagrams never enter the host DOM.
Host CSP allows inline scripts for inherited Blob/viewer JavaScript; preview CSP
and the opaque sandbox still restrict their capabilities.

Plan JSON may contain secrets. The generated diagram still contains resource
identifiers, and SVG may show validated route destination constants; HTML omits
those constants. See [route display](routes.md) and [provider evidence privacy](provider-evidence.md#privacy-and-validation).
Keeping generation local does not make the explicit download anonymous.

## Limits and verification

The library/file picker limits input to 32 MiB UTF-8, not total memory or duration.
Trees, graph expansion, search, strings/copies and preview need additional memory;
dense plans can exhaust Wasm/browser memory before a recoverable error. Cancel long
work, reduce input or use CLI selection. A fresh Worker drops prior high-water
memory, without guaranteeing immediate OS reclamation. Measurements report linear
memory capacity, not browser RSS or JavaScript heap.

検証コマンド・coverage・計測条件は [開発者向け検証](browser-development.md#verification) を参照してください。

## Per-generation memory selection

[初期化・上限・失敗時の lifecycle](browser-engine.md#per-generation-memory-selection) を参照してください。
