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

Choose SVG/HTML and configure the API options above. Remote-state mappings are API-only.
Diagnostic panels require HTML; see [viewer diagnostics](viewer.md#semantic-diagnostics).
Module/focus pickers inspect the original inputs, not filtered output. Pass selectors
verbatim; selection never bypasses generation validation. Clearing focus disables
depth; blank depth uses 1, zero includes no neighbors. Spatial options preserve the
same seven-type allowlist as the CLI.

Reset restores defaults and clears selectors while keeping files and completed
inventories. Replacing inputs clears inventories/selectors; other settings remain.
Settings edits cancel pending work; search/pagination do not. Retry reloads both
inventories, and stale results cannot replace newer ones. Unsupported input forms
include state JSON, event streams, binary plans and provider evidence; use the CLI
for Terraform execution and configuration/preset files.

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
