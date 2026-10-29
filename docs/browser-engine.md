# Browser engine integration

組み込み側の開発者向けです。[共通 API・options](browser.md#api-and-supported-options)、[named inputs](browser.md#named-state-inputs)、[privacy と Worker lifecycle](browser.md#privacy-and-lifecycle) と合わせて参照してください。[開発・検証手順](browser-development.md) は別文書です。

- [Standalone engine without artwork](#standalone-engine-without-artwork)
- [Resumable generation](#resumable-generation)

## Standalone engine without artwork

`sh scripts/build-wasm-engine.sh` (inside the development container) builds with
`--no-default-features`. Normal CLI and Wasm builds also exclude artwork.
The deprecated `icon-artwork` feature is a no-op. Service classification, labels
and layout use service-name mode.
An omitted `icon_mode` defaults to `service-name`; explicitly requesting `icon`
returns `invalid_options` (the native CLI also rejects it). Site integrations
must explicitly send `"icon_mode":"service-name"`.

One wasm-bindgen invocation writes `output/wasm-engine/planorama.js` and
`planorama_bg.wasm`; `manifest.json` records package/binding versions, compiled
revision (including dirty/unavailable), memory capabilities and SHA-256 hashes.
Revision comes from engine HTML metadata, so packaging works without Git and
cannot relabel an old engine with a new checkout SHA. Both builds append the same
memory adapter; the normal distribution has `pkg/memory-capabilities.json`.
`output/wasm-release/planorama-<version>-wasm-no-artwork.tar.gz` contains engine,
manifest and licenses, excluding UI/artwork; `.sha256` supplies its checksum.
Never mix archives' bindings/Wasm.

**Wasm engine release** is manual and build-only by default. Publication validates
both builds and requires an existing `v<package-version>` tag, published stable
GitHub Release and successful manual **Layout quality** for the checkout commit
(not a PR merge ref). Size/SHA-256-matching assets are reused; conflicts abort
before upload, and retries send only missing files. Normalized timestamps/owners/
modes make identical engine files reproducible. The workflow creates no tag/release;
publish only a validated release commit.

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
Serve the two engine files locally with the site using the [documented MIME types](browser-development.md#build-and-run).
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
responsive. The shared [error, inspection and generation contracts](browser.md#api-and-supported-options) also apply.

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

Rust uses `Generator::new(String, Options)` or
`Generator::new_named(Vec<NamedInput>, Options)` and
`step(usize) -> Result<Option<Generated>, Error>`. Futures retain placement,
candidates, queues, scoring and refinement state and resume without replay.
Dropping them cancels without a background executor/thread. Synchronous generation
drives the same implementation to completion without browser structure/search budgets.

Both job forms expose `stage()` and `step_with_progress(units, callback)`.
The latter shares `step`'s result/units and reports crossed boundaries without
retaining callbacks/history. Callbacks must return promptly and never re-enter;
Wasm exceptions are ignored, Rust accepts `Fn(&'static str) + 'static`.
Names are `parsing`, `selection`, `placement`, `routing`, `output`, with no
identifiers/values/percentages. `stage()` starts at parsing and retains the last
stage even after closure; it does not signal completion. Named jobs parse all,
select once, place/route each selected state, then compose. Refinement can repeat
stages, so progress is not monotonic. Callbacks reveal boundaries crossed inside
a step; scheduling, timers, stale-message rejection and Worker termination belong
to the caller.

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
| Module / spatial-type selections / remote-state mappings | 256 each | Before retaining the job |
| Combined module, spatial-type, focus and mapping string bytes | 64 KiB | Before retaining the job, including the Rust API |
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
node/edge, search-work and output limits across the entire input set. Cross-state
edges count toward the 1,024-edge budget before and after filtering, and resolution
yields between references and output sources. Budgets do not
reset per state or step. Grid/queue capacity limits apply to each individual
search. Options are shared by the whole job. There are at most 64 valid state
names (at most 4,096 ASCII name bytes in total), separately from the 2 MiB decoded
plan budget. The named Wasm envelope is limited to 12 MiB + 16 KiB before JSON
deserialization to allow escaping; decoded plan bytes must still fit 2 MiB.
Envelope decoding and each plan's streaming preflight/parsing are synchronous.
The structural preflight accepts the same leading UTF-8 BOM characters as the
plan parser. Input byte budgets still count those characters. Options byte and
count budgets also apply to synchronous generation. Synchronous generation
rejects an explicit `memory_budget_mib` with `invalid_options`.

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

Both modules import wasm32 memory with a 65,536-page (4 GiB) maximum. The adapter
uses module-required initial size and selected maximum 8,192–65,536 pages; growth
past it is refused before allocation, including during steps/string transfer.
Capacity includes static data, stack, allocator overhead, fragmentation and unused
space; it excludes JS strings/copies, compiler/runtime metadata, Worker/browser
overhead and preview DOM. `wasm.memory.buffer.byteLength` measures capacity,
not live allocations or process RSS.

Construction/checkpoints preserve 1 MiB headroom; grid expansion estimates storage,
and grid/distance/queue growth uses fallible reservations. This can reject work
despite reusable free blocks. Parsing, projection, some collections and output
still allocate infallibly/non-interruptibly: OOM may trap instead of
`memory_limit`, although the maximum holds. On a trap, terminate without any
further Wasm call (including `free`) and retry in a fresh Worker; never accept a
partial diagram.

Use one active job per Worker. Shared calls/jobs share one maximum, not additive
budgets. Free/cancel does not shrink memory or guarantee immediate OS reclamation;
lowering the budget requires a new instance. Input/reference/grid/queue/output/
preview limits stay independent, so `resource_limit` may occur at any memory
setting. Scheduling/rest/hidden-page policy is independent too.

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

After building the release CLI with `--no-default-features` and standalone engine,
run `node tests/browser/wasm-engine.cjs` to verify it with Chromium: archive contents,
provenance/hashes, JS/Wasm artwork exclusion, AWS/Terraform native SVG/HTML parity,
`viewer-plan` omitted/explicit icon options and archive reproducibility.
The focused native check is `cargo test --locked --no-default-features --lib artwork_free::`.
PR/full-library selection is defined in [CI](ci.md#browser-wasm-432), with
[coverage rationale and measurements](ci-history.md#browser-wasm-test-scope-audit-885).
