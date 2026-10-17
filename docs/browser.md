# Browser generation

The browser API accepts one `terraform show -json <saved-plan>` JSON string and
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
the browser boundary. Icons are compiled from the same bundled assets.

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

| Option | Values / default |
| --- | --- |
| `format` | `svg` (default), `html` |
| `address_format` | `qualified` (default), `terraform` |
| `icon_mode` | `icon` (default), `service-name` |
| `hide_relationship_legend` | Boolean, false |
| `changed_only` | Boolean, false |
| `focus` | Resource address or null |
| `focus_depth` | Nonnegative integer; requires focus; default neighbor depth 1 |
| `modules` | Array of module addresses, default empty |

Unknown keys, invalid enum values and invalid combinations fail explicitly.
The minimal UI exposes format and changed-only; the API supports all rows above.
State JSON, plan event streams, multiple states, saved binary plans, Terraform
execution, provider evidence, config/preset files, module listing, coverage/layout
report files and opt-in HTML diagnostic panels are unsupported in this API.

## Privacy and lifecycle

Only static assets are fetched at startup, without credentials. After the Ready
message the page holds the compiled Wasm module and Worker source in memory.
Generation, preview and download work with the network disabled while this page
remains open. Reloading requires fetching the assets again; there is no service
worker or persistent offline cache managed by the sample.

Input, output and diagnostics are never sent over the network or automatically
written to localStorage, sessionStorage or IndexedDB. Downloads are explicit.
Each request uses a new Worker; completion, cancellation and failure terminate
it. Cancellation also invalidates pending file reads. Blob URLs from prior results
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
termination releases its Wasm allocation; a fresh request does not retain its
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
repeated generation, cancellation/retry, sandbox isolation and empty browser
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
