# Browser development and verification

開発者・配布担当者向けです。[利用手順・API・privacy](browser.md)、[engine の組み込みと配布契約](browser-engine.md)、[CI 判定・cache 復旧](ci.md#browser-wasm-432) を参照してください。

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

## Verification

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
