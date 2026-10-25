# Browser generation

The browser API accepts a `terraform show -json <saved-plan>` JSON string or explicit named inputs and
returns SVG or self-contained HTML in memory. It uses the native CLI's parser,
semantic rules, selection, layout and rendering. JSON-only AWS accounts remain
unknown; no provider evidence or account inference is added.

## Build and run

```sh
docker compose up -d --build dev
docker compose exec -T dev sh scripts/build-wasm.sh
node scripts/serve-browser.cjs
```

Open `http://localhost:8080`, choose `tests/fixtures/viewer-plan.json`, select SVG
or HTML, generate, preview and download. The host server needs Node.js; generation
needs only a browser supporting WebAssembly, Web Workers and Blob URLs. To serve
inside Docker instead, publish port 8080 and set `HOST=0.0.0.0` explicitly.

`output/browser/` is the complete static distribution. Serve that directory from
any static host, including Cloudflare Workers Static Assets; `.wasm` should use
`application/wasm`, `.js` a JavaScript MIME type. No generation API, deployment,
domain, authentication or billing integration is included.

Rust 1.85.1, the Wasm target and wasm-bindgen-cli 0.2.100 are installed by the
Dockerfile. Cargo.lock fixes crate resolution, and Cargo.toml pins wasm-bindgen
to the same version as its CLI. The build script uses `--locked` and checks the
binding tool version. wasm-bindgen supplies the string/error ABI without custom
allocation exports. Its [no-modules output](https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html)
can be combined with the Worker script into a Blob, so recreating a Worker never
requires another network fetch. No npm build dependencies are needed.

`hcl-rs`, `zip`, `same-file`, `tempfile` and `toml` are native-only dependencies:
saved-plan/provider evidence, filesystem protection and CLI settings are outside
the browser boundary. All builds use service names without bundled artwork.

## Standalone engine without artwork

`sh scripts/build-wasm-engine.sh` (inside the development container) builds with
`--no-default-features`. Normal CLI and Wasm builds also exclude artwork.
The deprecated `icon-artwork` feature is a no-op. Service classification, labels
and layout use service-name mode.
An omitted `icon_mode` defaults to `service-name`; explicitly requesting `icon`
returns `invalid_options` (the native CLI also rejects it). Site integrations
must explicitly send `"icon_mode":"service-name"`.

The engine build writes `output/wasm-engine/planorama.js` and
`planorama_bg.wasm` from one wasm-bindgen invocation, plus `manifest.json` with
package version, source commit, binding version, memory capabilities and SHA-256 hashes. Packaging
produces `output/wasm-release/planorama-<version>-wasm-no-artwork.tar.gz` and
its `.sha256` sidecar. The archive contains the engine, manifest and software licenses,
without the sample browser UI or artwork. Do not mix bindings and Wasm files
from different archives. Both builds append the same memory initialization adapter
to the pinned bindings. The normal distribution also contains `pkg/memory-capabilities.json`.

The manual **Wasm engine release** workflow validates both normal browser and
standalone engine builds. Its default is build-only. Publication requires an
existing `v<package-version>` tag and published stable GitHub Release, plus a
successful manually dispatched **Layout quality** run for the checked-out commit
(a PR merge-ref check does not qualify). It attaches the
archive and checksum without replacing existing assets; matching existing assets
are verified by size and SHA-256 and reused, so retrying a partial upload sends
only missing files. Conflicting assets stop publication before any upload.
Archive metadata (timestamps, owners and modes) is normalized so rebuilding the
same engine files produces the same checksum. The workflow creates no tags or
releases. Run publication only after the release commit has passed validation.

After that version's assets have been published, fetch a pinned pair:

```sh
version=0.1.0
archive="planorama-${version}-wasm-no-artwork.tar.gz"
gh release download "v${version}" --repo ritz-p/planorama --pattern "$archive" --pattern "$archive.sha256"
sha256sum -c "$archive.sha256"
mkdir -p engine
tar -xzf "$archive" -C engine
```

Check `engine/manifest.json` against the expected version and commit; record the
archive checksum with the site dependency. The command describes the release
contract and does not imply that assets for a version have already been published.
Serve the two engine files locally with the site using the MIME types above.
No sample UI files need to be copied. A minimal consumer is:

```html
<script src="./engine/planorama.js"></script>
<script>
async function generatePlan(planJson, format = 'svg') {
  await wasm_bindgen({module_or_path: './engine/planorama_bg.wasm'});
  return JSON.parse(wasm_bindgen.generate(planJson, JSON.stringify({
    format, icon_mode: 'service-name'
  })));
}
</script>
```

Production integrations should invoke the same API in a Worker to keep the UI
responsive. The error, inspection and generation contracts below also apply.

## Resumable generation

`new wasm_bindgen.Generation(json, optionsJson)` owns a single-input generation.
`wasm_bindgen.Generation.named(inputsJson, optionsJson)` owns a named-input
generation with the same lifecycle. Call
`step(units)` with 1–4096 work units. It returns `undefined` while pending and the
same JSON result as `generate` on completion. `cancel()` drops all retained Rust
state and is idempotent; `free()` disposes the binding object. A step after
completion, failure or cancellation returns `generation_closed`. Invalid unit
counts, including non-finite or fractional JavaScript numbers, return
`invalid_options` without advancing the job. Always free the job;
terminate its Worker after completion, failure or cancellation to release the
instance's linear-memory high-water allocation.

The Rust equivalent is `Generator::new(String, Options)` and
`Generator::new_named(Vec<NamedInput>, Options)`, with
`step(usize) -> Result<Option<Generated>, Error>`. Compiler-generated Future state
machines retain placement preferences, route candidates, search queues, scoring
caches and refinement attempts. They resume at the pending checkpoint without
replaying completed work. The existing synchronous API drives this shared
implementation to completion, without the browser structure/search budgets.
Dropping the Future cancels it; no background executor or thread is retained.

Both job forms expose `stage()` and `step_with_progress(units, callback)`.
The latter has the same result and work-unit contract as `step`, and invokes the
callback at real boundaries crossed during that step. It retains neither the
callback nor an event history between calls. Wasm callback exceptions are ignored;
callbacks should return promptly and must not re-enter the job. Rust accepts an
`Fn(&'static str) + 'static` observer. Names are fixed: `parsing`, `selection`,
`placement`, `routing`, `output`. No identifiers, values or percentages are sent.
`stage()` initially returns `parsing`, then the most recently entered stage,
including after closure; it is not a completion indicator. Named jobs parse all
states and select once, then perform placement/routing for each selected state,
then compose output. Placement/routing may repeat, including internal refinement;
stages are not a monotonic progress scale. Use the callback to observe boundaries
that a larger step crosses before returning. Pending/rest/paused scheduling,
timers, stale-message rejection and Worker termination remain caller responsibilities.

Inside a fresh, initialized Worker, a named job can be driven with:

```js
const job = wasm_bindgen.Generation.named(JSON.stringify(inputs), optionsJson);
function tick() {
  try {
    const result = job.step_with_progress(64, stage => postMessage({stage}));
    if (result === undefined) setTimeout(tick, 0);
    else { job.free(); postMessage({result}); close(); }
  } catch (error) {
    // A trap can leave the instance unusable; the caller must discard this Worker.
    postMessage({failed: true}); close();
  }
}
tick();
```

The parent terminates the Worker on cancellation and after its terminal message.
Retry uses a fresh Worker. The synchronous sample UI remains on its existing API.

One unit crosses one cooperative checkpoint, not a fixed number of milliseconds.
Checkpoints cover placement refinement, route candidates, queue expansion, path
reconstruction and refinement passes. Input validation/parsing, semantic
projection, initial placement, sorting, grid allocations and output composition
still contain synchronous work. The following budgets bound those inputs; they
do not guarantee a device-independent maximum step duration.

| Resumable budget | Limit | Enforcement |
| --- | ---: | --- |
| Input UTF-8 bytes | 2 MiB | Before retaining the job |
| Options JSON UTF-8 bytes | 64 KiB | Before Wasm options deserialization |
| Module / spatial-type selections | 256 each | Before retaining the job |
| Combined module, spatial-type and focus string bytes | 64 KiB | Before retaining the job, including the Rust API |
| JSON values and keys / nesting | 32,768 / 64 levels | Streaming preflight, before building input trees |
| Parsed and projected nodes / edges | 256 / 1,024 | Before semantic expansion and layout respectively |
| Search grid points | 131,072 | Before grid and distance allocations |
| Search queue entries | 262,144 | Before queue insertion |
| Cumulative search work | 67,108,864 grid points plus queue pops | Across all searches, steps and states; charged before grid allocation and on queue removal |
| Output UTF-8 bytes | 4 MiB | After composition, before returning output |
| Wasm linear memory | 512 MiB default; 512 / 1024 / 2048 / 4096 MiB | Imported memory maximum, fixed at instance initialization |

Structural budget failures return `resource_limit`; memory preflight or fallible
allocation failures return `memory_limit`. Both discard the job and never return a
partial diagram. Projection and output checks occur after their respective
allocations; the earlier input/structure checks and Wasm memory maximum provide
additional protection. The native API retains its 32 MiB input limit. Wasm OOM
can trap rather than return a typed error; discard that Worker and retry with a
smaller input. Named jobs enforce the same input, structure, parsed/projected
node/edge, search-work and output limits across the entire input set. They do not
reset per state or step. Grid/queue capacity limits apply to each individual
search. Options are shared by the whole job. There are at most 64 valid state
names (at most 4,096 ASCII name bytes in total), separately from the 2 MiB decoded
plan budget. The named Wasm envelope is limited to 12 MiB + 16 KiB before JSON
deserialization to allow escaping; decoded plan bytes must still fit 2 MiB.
Envelope decoding and each plan's streaming preflight/parsing are synchronous.
The structural preflight accepts the same leading UTF-8 BOM characters as the
plan parser. Input byte budgets still count those characters. Options budgets
apply only to resumable generation. Synchronous generation retains its existing
options and rejects an explicit `memory_budget_mib` with `invalid_options`.

### Per-generation memory selection

`memory_budget_mib` is an integer option for Wasm `Generation`: 512, 1024, 2048
or 4096, default 512. Null, strings, fractions, zero, negative and other values
are errors. The shared Rust options decoder validates it; native Rust generation
rejects explicit memory selection because it cannot enforce a Wasm memory limit.
Read `JSON.parse(wasm_bindgen.memory_capabilities())` before initialization for
`allowed_mib`, `default_mib`, `maximum_bytes`, `initial_pages`, enforcement and
lifecycle metadata. `manifest.json.memory` contains the same capability object
for the standalone engine. A supported maximum is not a reservation guarantee
on every device.

Start a fresh Worker, initialize with the selected budget, then send that same
value in the generation options. For example, inside the Worker:

```js
const memory_budget_mib = 1024;
const wasm = wasm_bindgen.initSync({module, memory_budget_mib});
const job = new wasm_bindgen.Generation(planJson, JSON.stringify({
  memory_budget_mib, icon_mode: 'service-name', format: 'svg'
}));
```

The asynchronous initializer accepts the same option alongside `module_or_path`.
Omitting both settings selects 512 MiB. A generation/initialization mismatch or
attempt to change an initialized instance returns `memory_budget_mismatch`;
settings are never silently reduced. Supplying a custom `memory` is unsupported
by the distribution adapter. If the runtime cannot create the requested memory,
initialization returns `memory_unavailable`; discard that Worker. Other module
loading/instantiation failures also require a fresh Worker, with no fallback to
a lower budget. The sample synchronous UI has no memory selector; the site UI
is maintained separately.

Both modules declare an imported wasm32 memory with a 65,536-page (4 GiB)
maximum. The adapter creates memory with its small module-required initial size
and a selected maximum of 8,192–65,536 pages. The runtime refuses growth beyond
that maximum **before** allocating more linear memory, including inside a step
and during string transfer. This includes static data, stack, allocator overhead,
fragmentation and unused capacity; it is not all usable input/output storage.
It excludes JS strings/copies, runtime/compiler metadata, browser/Worker overhead
and preview DOM. Allocator live bytes are not measured. The reported
`wasm.memory.buffer.byteLength` is capacity, not live allocations or process RSS.

At construction and checkpoints, a conservative preflight keeps 1 MiB of linear
memory headroom. Grid expansion estimates additional storage before allocating;
grid/distance vectors and queue growth also use fallible reservations. These
checks may reject work even when the allocator could reuse existing free blocks.
Parsing, semantic projection, some collections and output serialization still
contain infallible allocation and non-interruptible work. Their OOM can trap
instead of returning `memory_limit`; the imported maximum still applies. On a
trap, terminate the Worker without further Wasm calls, including `free`, and
retry using a new Worker. No partial diagram is a successful result.

Use one active generation per Worker. Any jobs or synchronous calls sharing an
instance share its single memory maximum; there is no additive per-job allowance.
Freeing/cancelling a job does not shrink linear memory or immediately return
memory to the OS. A lower budget requires a fresh Worker/instance even after a
job finishes. Input, reference, grid/queue, output and preview limits remain
independent; `resource_limit` can therefore occur at any memory setting. Scheduling
units, rest intervals and hidden-page behavior are independent of this option.

The build verifies the binary import limits against the binding and capability.
The pinned wasm-bindgen uses unsigned pointers for JS views; Rust byte conversions
use `u64`, and JavaScript uses Numbers without 32-bit bitwise conversion. Individual
Rust allocations remain limited by `isize::MAX` and allocator/runtime constraints.
[V8's 4 GiB implementation](https://v8.dev/blog/4gb-wasm-memory) explains the
separate pointer and length boundaries; this does not promise 4 GiB availability
in every browser. Normal CI uses small inputs at every configured maximum and a
small imported-memory cap to exercise real OOM/trap recovery without allocating
GiB of data. `node tests/browser/memory.cjs [distribution-directory]` runs these
checks. Opt-in `PLANORAMA_MEMORY_STRESS=1 node tests/browser/memory-large.cjs`
tests allocator/string pointers above 2 GiB, a full 4 GiB buffer and high-water
refusal. Run it in an isolated container with an external timeout; it is excluded
from normal CI.

Consumers must run only one job per Worker, insert a real timer/task between
steps, pause at boundaries when hidden, and invalidate callbacks when cancelling
or changing settings. The site starts at 64 units, adjusts toward 4 ms observed
steps, and rests for at least 25 ms or nine times the preceding work duration.
This targets at most approximately 10% active time over sustained work, not a
fixed CPU percentage. Compilation, startup, host copies and preview are outside
that ratio. A watchdog should cover an active step, not elapsed time including
normal rests. Browser timer throttling can delay both work and watchdogs.

`node tests/browser/resumable.cjs` compares small-fixture SVG/HTML output hashes and reports
active/elapsed time, longest observed step, process CPU time, linear memory and
process RSS in isolated Node Workers using the distribution Wasm. Run it inside
Docker with an outer timeout; it also terminates each measurement after three
minutes. Set `PLANORAMA_STRESS=1` to include the longer Terraform fixture, and use
an isolated container with explicit CPU/memory limits for that opt-in run.
The report records runtime, CPU, total system memory and distribution file hashes.
Process CPU/RSS includes the Node host and compiler threads, and RSS is
an end-of-case sample, not a browser peak. It does not measure preview or claim
equivalence to a low-performance physical machine. Browser/site measurements
must additionally cover transfer, preview, UI response, cancellation and retries.

An initial Docker/Node 18 observation on `terraform-plan.json` (15,740 bytes)
reduced uninterrupted SVG generation from 2.51 s to a longest observed step of
22.3 ms, with 3.29 s active work over 83.9 s elapsed and 12 MiB Wasm memory.
HTML's longest observed step was 67.5 ms. These are single observations during
development, not calibrated capacity guarantees or completed site#30 results.
Budget and scheduling choices remain conservative provisional values until
representative browser stress measurements and low-performance hardware results
are available.

Verify the extracted release archive in Chromium with
`node tests/browser/wasm-engine.cjs` after building the release CLI (with
`--no-default-features` for the standalone configuration) and
the standalone engine. It checks archive membership, provenance and hashes,
scans JS/Wasm for embedded artwork markers, compares AWS and Terraform SVG/HTML
outputs exactly with native service-name generation, and tests omitted/explicit
icon options and archive reproducibility. `cargo test --locked --no-default-features --lib`
runs the complete library suite, including the native API contract. The same artwork-exclusion contract applies with all features enabled.

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

Unknown keys, invalid enum values and invalid combinations fail explicitly.
Both diagnostic options reject SVG with `invalid_options`; the form explains
the conflict and focuses the enabled diagnostic checkbox. Switching formats
retains checkbox values. Reset clears both.
The UI exposes all rows above with the same defaults. Generation settings remain
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
mapping diagnostics and provider evidence are unsupported; this is not complete
analysis. Only existing identifiers and fixed explanations are embedded, without
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
[resumable generation](#resumable-generation); its aggregate limits are smaller.

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
and SVG/HTML rendering. Explicit remote-state mappings and provider evidence are
not exposed by this API; use the CLI for those features. Equal names or ARNs do
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

```sh
docker compose exec -T dev cargo build --locked --release --bin planorama
docker compose exec -T dev sh scripts/build-wasm.sh
docker compose exec -T dev node tests/browser/wasm.cjs
```

`Browser Wasm` runs these steps in CI. The browser test requires exact native/Wasm
output equality (including identity, action, relationships and geometry), covers
SVG/HTML downloads, selections, invalid input/options, markup-bearing addresses,
all UI defaults/changes/reset, empty/single/multiple spatial selections, focus/depth validation, special-character module
rows and duplicates, native/Wasm inventory counts, root/empty/configuration rows,
data/deposed objects, resource-picker/manual/native focus parity, ambiguous and
non-rendered addresses, inspection without generation stages, search/pagination
and selection retention, keyboard focus,
safe error announcements, settings changes
during file reads and Worker execution, stale completions, repeated generation,
cancellation/retry, sandbox isolation and empty browser
storage. After asset loading it disables network access and asserts zero HTTP
requests through Chrome's network log. Large-fixture elapsed time, linear memory
and output sizes are printed as observations, without timing thresholds in CI.

One run on 2026-10-18 in Docker Desktop Linux, Rust 1.85.1, Chromium
155.0.8059.79 (with native tests running concurrently) measured:

| Fixture / output | Input bytes | Generation | Wasm linear memory | Output bytes |
| --- | ---: | ---: | ---: | ---: |
| `viewer-plan.json` / SVG | 1,565 | 36.1 ms | 1.56 MiB | 40,558 |
| `viewer-plan.json` / HTML | 1,565 | 4.5 ms | 2.19 MiB | 112,082 |
| `terraform-large/plan.json` / SVG | 52,927 | 31.53 s | 44.06 MiB | 332,046 |

Each run used a fresh Worker. Timings include parsing through serialization and
exclude asset loading, Worker startup, transfer and preview. Small-result timing
differences include browser JIT warmup; these are observations, not a comparison
of SVG versus HTML performance or device-independent capacity claims.
The large fixture contains 50 resource changes; even this input can take tens of
seconds because routing cost depends on relationships and geometry. The compiled
Wasm asset was 3,139,308 bytes before HTTP compression.
