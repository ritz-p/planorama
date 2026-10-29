# CI measurement and coverage history

[現行の CI 判定・復旧手順](ci.md) に戻る。計測結果は記録した版・環境・cache 条件の観測であり、現在の所要時間の保証ではありません。判断理由・比較条件・再現手順はここで保存します。リンク先の実行 artifact には保存期限があるため、この記録も維持してください。

- [Observed timing and runner cost](#observed-timing-and-runner-cost)
- [Local timing](#local-timing)
- [Test runner evaluation (#254)](#test-runner-evaluation-254)
- [Calibration split timing (#406)](#calibration-split-timing-406)
- [Browser Wasm comparison, 2026-10-26](#browser-wasm-comparison-2026-10-26)
- [Browser Wasm test scope audit (#885)](#browser-wasm-test-scope-audit-885)

## Observed timing and runner cost

The 2026-10-12 [serial baseline](https://github.com/ritz-p/planorama/actions/runs/38224372411)
and [parallel run](https://github.com/ritz-p/planorama/actions/runs/38225451104)
used identical Rust sources, fixtures, toolchain and dependency cache. Active
workflow time fell from 170 to 141 seconds, while summed job time rose from
170 to 201 seconds. This single comparison supports two suites as a latency/cost
tradeoff, not a performance guarantee. Ordinary integration tests stayed in
standard because their measured execution was only 1.40 seconds.

## Local timing

The [#253 comparison](https://github.com/ritz-p/planorama/pull/410) measured local
standard-suite harness time falling from 119.49 to 70.98 seconds after deferring
three repeated large-fixture tests. Their separate two-thread run took 140.201
seconds. This moves work off PRs; it does not eliminate it or establish hosted
end-to-end savings.

## Test runner evaluation (#254)

The 2026-10-15 two-repeat comparison retained `cargo test`: nextest was slower
for standard by 3.6% at two threads and 1.4% at eight; Large differed by less than
one second. Revisit if suite distribution changes. [PR #411](https://github.com/ritz-p/planorama/pull/411)
records the decision; [original measurements](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/ci.md#test-runner-evaluation-254)
retain per-run CPU/timings, inventories and failure probes.

Conditions: post-#253 selection at `efb7de9`, Rust 1.85.1, optimized test profile,
cargo-nextest 0.9.148, Docker Desktop Linux, Ryzen 9 5950X, 32 visible CPUs, no
container quota. Test concurrency varied, not CPU allocation; sources/fixtures
were identical. Historical results used the former Python tool, not its JavaScript
replacement, and do not predict current runtime.

The [comparison script](../tests/support/compare_test_runners.cjs) reuses CI selection,
checks libtest names/ignored flags and rejects unexpected nextest filters. It
compiles before timing, alternates runners and reverses concurrency order on the
second repeat; ignored benchmarks/deferred regressions stay excluded. JSON records
wall time, child-process CPU, revision, tools, inventories and logs. Node
`performance.now()` measures elapsed time; GNU `time` measures descendant user/system
CPU (0.01 s precision). Failed runs retain stdout/stderr logs before nonzero exit.
Only Node built-ins and Docker's `time` package are needed.

After downloading nextest, run
`docker compose exec -T -e NEXTEST_BIN=/tmp/planorama-nextest dev node --test tests/support/tooling.test.cjs`.
Its temporary dependency-free crate checks ignored inventory, order and failure
logs for both runners; these are tooling probes, not performance measurements.

Reproduce inside the existing development container from the repository root:

```sh
docker compose exec -T dev sh -c 'mkdir -p /tmp/planorama-nextest && curl -LsSf https://get.nexte.st/0.9.148/linux -o /tmp/planorama-nextest/archive.tar.gz && tar xzf /tmp/planorama-nextest/archive.tar.gz -C /tmp/planorama-nextest'
docker compose exec -T dev node tests/support/compare_test_runners.cjs --suite standard --threads 2 8 --repeats 2 --nextest-bin /tmp/planorama-nextest --output output/nextest-standard-runs
docker compose exec -T dev node tests/support/compare_test_runners.cjs --suite large --threads 2 --repeats 2 --nextest-bin /tmp/planorama-nextest --output output/nextest-large-runs
```

Use a new output directory for each experiment; the script refuses to overwrite
previous evidence. The downloaded binary is temporary and is not a Docker image
or CI dependency. See nextest's [binary installation](https://nexte.st/docs/installation/pre-built-binaries/)
and [test inventory format](https://nexte.st/docs/machine-readable/list/).

The measurement excludes download, cold compilation, checkout, queueing and
cache transfer. Both runners passed failure-output and ignored-test compatibility
probes; measured performance was the reason for retaining Cargo.

## Calibration split timing (#406)

[PR #408](https://github.com/ritz-p/planorama/pull/408) records identical-source,
warm-dependency Docker measurements: standard command time changed from 242.355
to 166.437 seconds, with calibration alone costing 74.663 seconds. The single
comparison excludes hosted setup, queue, transfers and the large suite; a manual
calibration run still pays its own setup, compilation and artifact costs.

## Browser Wasm comparison, 2026-10-26

[PR #675](https://github.com/ritz-p/planorama/pull/675) records the original cache
comparison and hosted phase timings. Complete job time was 707 seconds before,
885 seconds with initial cold caches, and 585 seconds with fully warm caches.
Initial BuildKit export added cost; warm reuse reduced it. These single-run
measurements predate test consolidation and do not establish current latency.
[The original local measurements](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/ci.md#browser-wasm-comparison-2026-10-26)
retain environment, phase and cache-size details.

## Browser Wasm test scope audit (#885)

The 2026-10-28 audit uses Rust sources and fixtures from `9da190b`. The deprecated
`icon-artwork = []` feature is a no-op: there are no feature predicates in Rust
sources or feature-environment checks in `build.rs`. `default = []` is also empty.
This is an inspected property of this revision, not an assumption that
`--all-features` and `--no-default-features` are always equivalent.

| Candidate | Decision and coverage |
| --- | --- |
| Repeat the complete no-default-feature library suite | Run `artwork_free::` (two tests) on PRs. They cover direct rendering, SVG/HTML defaults, missing artwork and explicit icon rejection. Keep no-default CLI help, native/Wasm builds, memory, archive dependency/byte scans, manifest/provenance, reproducibility and extracted-engine parity. Normal Rust CI retains the complete all-feature suite. |
| Split browser and standalone into jobs | Keep one required `browser` job. Independent jobs duplicate environment setup, Cargo restore and shared compilation; transferring built products adds a serial prerequisite and revision/feature matching requirements. The dominant duplicate library execution can be removed without those costs. Both distributions still fail the same check and use freshly built products from the same checkout. |
| Consolidate service fixture SVG/HTML parity | Retain all cases. Native semantic tests overlap in graph assertions but cannot detect Wasm target, ABI, worker serialization or output differences. Instrument individual fixture/format costs before considering a narrower representative set. |
| Move large completion to scheduled/manual CI | Retain it on every PR along with progress, cancellation and retry. Moving completion would delay detection of large-input Wasm traps, deterministic output differences and missing progress stages. |
| Bindings/cache changes | Keep the existing caches and both bindings builds; binding generation is below one second per distribution in the sampled runs. |

The selection skips 898 of 900 previously passing no-default library tests;
eight ignored tests were already excluded. Browser assertions/fixture-format pairs
are unchanged. Feature-specific code outside `artwork_free` requires re-auditing
this selection; no equivalence is promised for future feature combinations.
Full-library investigation and release coverage follow [current CI](ci.md#browser-wasm-432).

The browser inventory is unchanged: `viewer-plan` SVG/HTML and selection options;
15 complete service fixtures in both formats (DynamoDB, ElastiCache, EventBridge,
MSK, MQ, DocumentDB, Neptune, Redshift Serverless, App Runner, Network Firewall,
OpenSearch Serverless, Batch, FSx Lustre, DMS, SageMaker); four incomplete fixtures
(OpenSearch Serverless, Batch, DMS, SageMaker) in both formats; aliases, spatial
identity/SSM, basic/service labels; named inputs, module/resource selection and
options; malformed/unsupported/oversized inputs; offline downloads, isolated and
hostile previews; large-input progress/cancel/retry and full native/Wasm parity.
The semantic counterparts are in `tests/unit/semantic`, rendering checks in
`tests/unit/svg` and `tests/integration`; they do not replace browser boundary tests.

[PR #886](https://github.com/ritz-p/planorama/pull/886) records the controlled
full/focused comparison, final checks and warm-cache results. Hosted cold job
time fell from 782 to 570 seconds; warm time fell from 721 to 453 seconds.
Runner variability also affected unchanged browser work, so the whole difference
cannot be attributed to filtering. These are observations, not a ten-minute SLA.
[The original audit](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/ci.md#browser-wasm-test-scope-audit-885)
retains the earlier runs, local phases and browser breakdown.

To reproduce, use separate empty `CARGO_TARGET_DIR` and `SCCACHE_DIR` directories
for each cold mode and isolate sccache servers when sharing a container. Run
`WASM_CI_FULL_LIB=1 sh scripts/wasm-ci.sh` and
`WASM_CI_FULL_LIB=0 sh scripts/wasm-ci.sh`, selecting distinct
`WASM_CI_REPORT_DIR` paths. For each warm repeat, remove only workspace products
and CLI test binaries with the workflow's cache-preparation command, reset
sccache statistics, and retain dependencies. Record environment and complete
job durations alongside the TSV files; do not compare cold full and warm focused
runs as if scope were their only difference.
