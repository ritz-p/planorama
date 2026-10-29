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


For the complete CI path run `sh scripts/wasm-ci.sh` inside Docker. Browser tests
compare native/Wasm SVG and HTML, exercise selection, cancellation and retries,
and verify sandbox/storage isolation with networking disabled after asset loading.
Timings and linear-memory capacity are observations, not performance thresholds.
