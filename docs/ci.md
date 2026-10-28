# CI validation and recovery

Native CLI distribution has a separate [release workflow and recovery procedure](releases.md).
Native release builds run only through `workflow_dispatch`, never automatically on
PR updates or tag pushes. Manual runs verify artifacts by default; publication
requires explicit `publish: true` on a matching existing version tag. Lightweight
release-control tests remain part of Workflow Tests on ordinary PRs.

Contributors and Codex regenerate maintained diagrams locally when rendering or
generation inputs change, inspect the diff, and include the outputs in the PR.
**Update SVG examples** is manual-only. When requested, it can append SVG changes
to the same PR branch, dispatch validation on that head, and publish results
through **Report PR validation**.

## Resource support catalog

**Resource support** runs `sh scripts/update-resource-support.sh --check` for
changes to provider/runtime sources, catalog generation, its Markdown output and
build inputs, plus the pinned AWS inventory, extractor and baseline comparator. It compares deterministic
Markdown, checks every managed type's basic capabilities and runs the offline
extractor and synthetic schema-diff regressions. It performs no SVG generation, routing or Provider download.
The Rust checks also belong to the standard Rust suite.
See [regeneration instructions](../CONTRIBUTING.md#リソース対応一覧の更新).

The same job checks the [semantic gap register](aws-semantic-audit.md) offline with
`node scripts/aws-semantic-audit.cjs --check` and
`node --test tests/support/aws-semantic-audit.test.cjs`. Schema evidence is pinned by
SHA-256; candidate paths do not establish runtime semantic coverage. Audit-only
changes do not require SVG regeneration or Layout quality.

## Toolchain and dependency cache

Local setup and check commands live in [CONTRIBUTING](../CONTRIBUTING.md).
Ordinary Formatting, Tests and Clippy use the pinned `rust-toolchain.toml` on
Ubuntu runners. Tests and Clippy cache Cargo registry/git data and dependency
build products with `Swatinem/rust-cache`; workspace crates are rebuilt each run.
Tests use the `v2-mold-2.40.4` prefix for the pinned Linux x86_64 native linker;
Clippy retains `v1-rust-dependencies`. Both exclude older workspace-product caches.
Keys include the runner/compiler environment, Cargo manifests and lockfile,
toolchain/Cargo configuration, build-related environment and the `build.rs` /
`assets/**` hash. Test suites share the `-tests` key, including installer/wrapper
hashes; Clippy uses a separate key.
Cargo still checks rebuild requirements, and cache misses do not reduce coverage.

Cargo test additionally uses checksum-verified sccache 0.10.0 with incremental
compilation disabled in those two jobs and Browser Wasm. Its 2 GiB local compiler cache is
restored separately from the existing dependency target cache. Target hits avoid
compiler requests; sccache is a fallback for eligible requests after invalidation
or a missing target cache. Only successful runs with newly written compiler
entries upload a new cache, after stopping the server. Unchanged cache hits do
not repeatedly upload identical data. Fork/PR cache scopes and workflow permissions
are unchanged. Clippy retains its existing target cache without this wrapper.
See [measurements and recovery](build-performance.md#compiler-cache).

Tests install checksum-verified mold 2.40.4 and select its compiler-driver wrapper
only through `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER`. Docker Compose uses
the same selection. Wasm retains its target rustflags and `wasm-ld`; native
release publication, cross compilation and other architectures keep their existing
linkers. See [scope, reproduction and measurements](build-performance.md).

MSRV uses Docker with Rust 1.85.0. Its workflow runs for changes to Cargo.toml,
Cargo.lock, rust-toolchain.toml, Dockerfile, .dockerignore, compose.yaml, the MSRV
workflow or shared validation-checkout helper. See [post-merge validation](#validation-after-merging)
for cache population and direct pushes to main.

## Cancelling superseded validation

**Cargo fmt**, **Cargo clippy** and **Cargo test** use workflow-level concurrency.
A new `pull_request` run cancels pending or running PR-event validation for the
same workflow and PR. Fixed `cargo-fmt`, `cargo-clippy` and `cargo-test` prefixes
separate the workflows even if their display names change. PR numbers separate
unrelated PRs, including forks whose source branches have the same name.
Completed runs and their check results remain visible in the run history.

Only `pull_request` events share a `pr-<number>` key. Every `workflow_dispatch` and
post-merge `pull_request_target` run uses a unique `run-<run_id>` key. This includes
PR-bound dispatches after automatic SVG commits: a dispatch request can arrive
after the PR has advanced, and must not cancel a newer PR check before its own
checkout rejects the stale SHA. The dispatched checkout and reporter still verify
the exact expected head. Repeated manual runs on the same branch also remain
independent. The existing non-cancelling SVG generation/writeback and
status-reporting groups retain their own policies.

See GitHub's [workflow concurrency documentation](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).

## Parallel Rust test suites

**Cargo test** starts two independent jobs with `fail-fast: false`, so a failing
suite does not cancel the other suite's coverage:

| Suite job | Targets |
| --- | --- |
| `Test suite (standard)` | Unit tests, ordinary integration tests, libraries, binaries, examples and benchmarks other than the large integration target. |
| `Test suite (large)` | The `planorama` package's `large_example` integration target. |
| `Tests` | A small aggregate job that succeeds only after both suites succeed. |

Both suites keep separate **Compile tests** and **Run tests** steps. The
[suite selector](../.github/scripts/test-suites.cjs) reads
`cargo metadata --locked --all-features --no-deps --format-version 1`, selects
Cargo's default workspace packages and builds explicit target arguments. The
compile step adds `--no-run` to the same `cargo test --locked --all-features`
commands used by the run step. Build scripts remain Cargo-managed prerequisites.
Snapshot comparisons remain in their original suites. Three explicitly ignored
large-fixture unit regressions run separately as described below.

Spacing-budget calibration is an ignored tuning experiment, run only on demand
by **Spacing budget calibration**. Standard and large retain ordinary routing
correctness and layout regression tests; the aggregate **Tests** check depends
only on those two suites. Calibration and deferred regression uploads are not PR gates.

New integration targets automatically join `standard`; no ordinary target list is
duplicated in the workflow. New library, binary, example and benchmark targets are
also selected. CI deliberately selects every discovered non-build target even
when its manifest has `test = false`, preserving targets such as examples that the
previous `--all-targets` command covered. An explicitly disabled integration or
benchmark target added later can therefore receive extra coverage. Unknown target
kinds, incomplete metadata, or a renamed/missing/ambiguous `large_example` target
fail explicitly and require updating the selection policy.

The two jobs share the existing dependency cache through a `shared-key` containing
the `build.rs`/`assets/**` and linker installer/wrapper hashes followed by `-tests`.
Clippy retains its separate cache. The pinned compiler, Cargo configuration,
manifest/lockfiles and build inputs still determine compatibility. Workspace
crates are rebuilt in each job and are not cached. Thus parallel execution can
shorten waiting time while increasing total runner time. Normal integration tests
stay with unit tests to avoid paying another runner startup and compilation for a
small amount of test execution. There is no serial artifact-upload prerequisite.

For comparisons, record both workflow wall time and the sum of the two suite jobs
plus the aggregate job, including checkout, toolchain/cache work and compilation.
Separate queue time from active job time, and record cache hits, per-step timings,
source/base SHAs and test counts. Compare identical Rust sources and fixtures;
runner elapsed seconds measure compute use, not an account's billed price.

### Observed timing and runner cost

The 2026-10-12 [serial baseline](https://github.com/ritz-p/planorama/actions/runs/38224372411)
and [parallel run](https://github.com/ritz-p/planorama/actions/runs/38225451104)
used identical Rust sources, fixtures, toolchain and dependency cache. Active
workflow time fell from 170 to 141 seconds, while summed job time rose from
170 to 201 seconds. This single comparison supports two suites as a latency/cost
tradeoff, not a performance guarantee. Ordinary integration tests stayed in
standard because their measured execution was only 1.40 seconds.

## Layout quality comparison (#390)

For opt-in paired-input resource displacement measurements, see
[layout stability](layout-stability.md). It has no automatic workflow or blocking
threshold and does not update the quality baselines below.

**Layout quality** runs on manual dispatch or PRs carrying the `layout-quality`
label (including subsequent pushes). It is opt-in and independent of **Tests**;
do not make this conditionally absent job a repository-wide required check.
Manual dispatch becomes available after the workflow reaches the default branch.
Its read-only job builds the pinned release renderer once, then renders six
existing inputs sequentially: small, large, three-way fan-in/bundling, cross-state,
account configuration and regions. JSON-only account configuration retains unknown
account evidence; it does not claim saved-plan account resolution coverage.
The expensive concentrated account-high-fan-in stress input is excluded; deferred
routing regressions retain their separate non-blocking lane below.

The same job runs [presentation geometry checks](presentation-quality.md) in
Bookworm Chromium against these generated SVGs and focused card fixtures.
Text/badge overlaps and content overflow fail independently of routing budgets;
browser observations are retained as `presentation.json` and `presentation.md`,
alongside the routing artifacts. The Rust diagnostic schema is unchanged.

Historical baseline decisions are recorded in their implementation PRs. These
are accepted geometry tradeoffs, not additional budgets or permission to offset
regressions against improvements elsewhere:

| Change | Fixture and reviewed consequence | Evidence |
| --- | --- | --- |
| #648: preserve header width during port-height reservation | Large: 11 newly affected crossing pairs / 13 intersections after fixing hidden gateway headings. Border deficits unchanged; bends within budget. | [PR #661](https://github.com/ritz-p/planorama/pull/661) |
| #666: remove the 32px role-mark reservation | Large: 28 crossing-pair increases / 31 intersections and three parallel-pair increases. Includes a 0.5px visible gap over 1,594px (22,316 px² deficit); explicitly accepted for narrower headers. | [PR #671, individual pairs](https://github.com/ritz-p/planorama/pull/671) |
| #667: shorten labeled headings | Large: 24 crossing-pair increases / 25 intersections despite the aggregate decrease. A 2.25px gap over 600px adds 7,200 px² parallel deficit. | [PR #672, individual pairs](https://github.com/ritz-p/planorama/pull/672) |
| #660: increase named-state heading allocation from 40 to 76px | Multi-state: three paths lengthen by 36px; two parallel pairs each add 144 px² (total 3,680 → 3,968). Other gated metrics unchanged. | [PR #663](https://github.com/ritz-p/planorama/pull/663) |

Each change updated only the named fixture baseline and retained all budgets.

The [manifest](../tests/layout-quality/manifest.json) is the ordered fixture and
budget authority. Each checked-in baseline records exact CLI arguments, SHA-256
input bytes, budgets, comparison format version, renderer contract
`planorama-final-layout-v1`, and the complete schema-1 final-layout report.
The renderer contract describes interpretation, not a source commit: ordinary
renderer changes must remain comparable. Change it when interpretation changes;
new diagnostic schemas require explicit comparator support before rebaselining.
Changed bytes (including input reordering), options or budgets require an explicit
update. A separate CLI test proves reversed resource/configuration ordering and
named-state argument ordering give identical diagnostics. Stable edge IDs and their
lexicographic UTF-8 byte order (matching Rust, including Unicode IDs) must match;
each ID must parse as JSON equal to its edge's structured identity.
Duplicate IDs, missing/non-array findings, incomplete findings/pair analysis, unsupported diagonals, policy or
schema mismatches fail as non-comparable, never as improvements.

Every fixture currently selects the following budgets independently. For each
measurement, permitted increase is `absolute + baseline * relative`; all initial
relative budgets are zero. A numerical tolerance of at most `0.000001` absorbs
floating-point comparison noise; values in artifacts retain their original
precision. Border diagnostics already round to quarter-pixel clearance units.

| Measurement | Policy | Absolute increase |
| --- | --- | ---: |
| Visible crossings | Gate | 0 |
| Independent overlap length (px) | Gate | 0 |
| Sustained parallel deficit (px²) | Gate | 0 |
| Minimum border deficit (px²), total and each stable edge | Gate | 0.25 |
| Raw bends on each stable edge | Gate | 2 |
| Canvas area (px²) | Informational | unrestricted |
| Raw length on each stable edge (px) | Informational | unrestricted |

Area can legitimately increase to improve readability. Raw length is not route
stretch: diagnostics do not expose a valid shortest route, so this comparator
does not invent one. Existing deferred stretch tests remain authoritative.
Shared bundle trunks are excluded from independent overlap. No aggregate score
offsets a bad fixture or edge with improvements elsewhere. Basic routing,
containment, snapshots and ordinary correctness tests remain independent.
Crossings, overlap and parallel deficits are checked both in total and for each
unordered pair of stable edge IDs, using the same absolute/relative budget on
each pair's baseline. Contributions come from the existing findings: crossing
counts, overlap interval lengths, and parallel interval lengths multiplied by the
reported centerline preference minus separation. Visible conflicts are not reanalyzed.
Before comparison or baseline updates, these contributions must match each
summary total within `0.000001`; missing or excess contributions make either
report non-comparable even when it declares complete findings.
All required summary measurements, including routed-path counts, raw length/bends
and shared-trunk length, must be present as finite nonnegative numbers.
Canvas width and height must themselves be finite nonnegative JSON numbers
before area is calculated; coercible strings and null are rejected.
Routed-path count must equal the number of edge entries. Aggregate raw length
and bends must equal their per-edge sums within `0.000001` before comparison
or baseline updates, so stale edge values cannot conceal an aggregate change.
Raw measurements are also checked against the serialized polyline: zero-length
segments are ignored, consecutive runs in the same direction add no bend, and
direction reversals do. Diagonals are non-comparable even if the declared
unsupported count is zero. This is a report-consistency check; visible conflict
measurements continue to come from the renderer's existing diagnostics.
The minimum-border-deficit summary must likewise equal the per-edge sum.
Both completeness flags must be the JSON boolean `true`; truthy strings and
other non-boolean values are rejected.
Segment positions and finding order do not define pair identity. A newly affected
pair starts at zero, so improvements on another pair cannot hide its regression.

Run or update locally inside Docker:

```sh
docker compose exec -T dev cargo build --release --locked --bin planorama
docker compose exec -T dev node scripts/layout-quality.cjs check target/release/planorama output/layout-quality
docker compose exec -T -e LAYOUT_QUALITY_BINARY=target/release/planorama dev node --test tests/ci/layout-quality.test.cjs
# Only after reviewing the changed inputs, geometry and intended budgets:
docker compose exec -T dev node scripts/layout-quality.cjs update target/release/planorama output/layout-quality-update
git diff -- tests/layout-quality
```

Maintainers reviewing routing/layout PRs own the budgets. Baseline changes require
a PR explanation identifying intentional geometry/input/schema changes and why
any increased budget is acceptable. Inspect the full JSON and rendered diagrams
before accepting changes; do not refresh baselines simply to clear a regression.
Updates write baselines only after every fixture renders with comparable complete
diagnostics. Missing/stale baselines fail checks. CI never updates or commits them.
Each render removes its prior report first and must produce a new regular file;
a successful process exit without fresh diagnostics cannot pass or update a baseline.
The same freshness and regular-file checks apply to the SVG preview. Fixture IDs
must be unique lowercase alphanumeric/hyphen names before any output is written.
The manifest filename and `results` are reserved. Empty fixture lists, malformed
input/argument lists and missing or invalid budgets are rejected before rendering,
including per-edge budgets for fixtures with no edges.
Pair findings require nonnegative integer segment indices and category-specific
location fields with finite JSON numbers; numeric strings are not accepted.
All finding segment indices, including peer and border references, must be less
than the referenced polyline's point count minus one.
Each routed polyline requires at least two finite two-coordinate points. State
geometry/card counts, policy fields, collection limits and informational border
measurements are also validated, including border-finding geometry.
Output and baseline directories must be separate: identical directories and either
ancestor/descendant relationship are rejected before creating output or invoking
the renderer. This check resolves directory symlinks, including existing parents
of a new output directory, so aliases cannot overwrite checked-in baselines.
Before any removal, all report, SVG, log, aggregate and baseline destinations are
checked against every declared input and the manifest, including symlink and
hardlink aliases. Baseline destinations must be regular files when present.
Updates stage each baseline in its own directory before an atomic rename; they
do not truncate existing files on serialization/write failure. All rendering must
succeed before this phase. The set of baseline renames is not a filesystem
transaction: interruption between renames can leave a partial update, which must
be rerun and reviewed. Concurrent mutation of inputs/output directories by another
process is outside this local workflow's guarantees.

The job publishes a bounded Markdown summary with deltas and stable relationship
identities, and retains full reports, comparison details, renderer logs and preview
SVGs for 14 days. Preview SVGs live only in the output artifact.
Markdown is capped at 60,000 characters, with at most 2,000 characters per error;
fixture statuses are listed first and detail space is divided between fixtures,
so a verbose early fixture cannot remove later failures.
Renderer logs retain captured process output. Per-edge border changes show only
border findings; raw bend/length changes do not attach unrelated spatial findings.
Prior artifacts for all selected fixtures and aggregate results are removed before
reading inputs, so input failures cannot retain an earlier preview. Log and aggregate writes
replace old files without following pre-existing file symlinks.
The maintained SVG generation script is not run. Failures keep a failing job status. The job has
a 35-minute ceiling, with 5 minutes for compilation, 2 minutes for comparison
tests and 20 minutes for the six-fixture comparison. Each render has a 3-minute
timeout, so even six sequential timeouts leave 2 minutes to finalize reports
within the comparison step and 8 minutes of job headroom for setup/publication.
Compilation and six layouts
are additional opt-in cost; performance timings are never quality gates.
Comparator tests run in the existing Workflow tests suite, and the opt-in lane
also runs the real CLI input-order test.
On 2026-10-15, Docker Desktop Linux with Rust 1.85.1 took 31.30 seconds for
the six-fixture check, excluding compilation and including container startup.
This is one local observation, not a CI time threshold or hosted-runner estimate.

## Terraform compatibility (#525)

The separate [Terraform compatibility matrix](terraform-compatibility.md) uses
committed-fixture checks on relevant PRs and fixed-tool offline capture only on
manual dispatch. It does not replace Tests, SVG regeneration or Layout quality.
Its separate `evidence` job also runs on relevant PRs: it prepares fixed Terraform
1.9.8 and 1.16.6 / AWS 5.100.0 installations, builds the CLI in Docker, then
verifies real saved plans against a local STS fixture with external networking
disabled. See the independent [provider evidence contract](provider-evidence.md).

## Deferred routing regressions (#253)

**Routing regressions** runs after every PR merge into `main`, nightly at 18:23 UTC
(03:23 JST), and on manual dispatch. Post-merge checkout uses the event's default
branch commit, never the PR head. Scheduled runs use the default branch; manual
runs use the selected revision.

These three `deferred_regression_` unit tests are excluded from ordinary Cargo
runs and selected by this workflow:

| Test suffix | Reason for deferral | Focused coverage retained on PRs |
| --- | --- | --- |
| `large_fixture_limits_each_edges_bends_and_valid_route_stretch` | Two full large-fixture layouts plus per-edge shortest-route searches and quality comparison after edge reversal. | `container_port_stubs_are_not_immediately_retraced`, `symmetric_peers_keep_ports_and_routes_when_edges_are_reordered`, small exhaustive candidate comparisons, and spacing/shape/border guards. |
| `congested_container_routes_improve_over_shortest_paths` | Large-fixture layout followed by both shortest and scored routing passes. | `simple_routes_respect_ports_obstacles_and_edge_conflicts`, scoring/overlap tests, `crossings_do_not_force_clear_detours`, and `completed_paths_compact_stubs_before_scoring_and_occupancy`. |
| `large_listener_locality_preserves_geometry_and_routes` | Three large-fixture layouts comparing owner locality, area and repeatability. | `multiple_owners_and_shared_targets_keep_stable_separate_cards`, `listener_owner_resolution_is_conservative`, and focused leaf locality/containment tests. |

The large-fixture-specific thresholds remain in deferred coverage. PRs retain
`large_aws_example_keeps_cross_module_containment_and_multi_subnet_relationships`
and the `large_example` integration target, including full SVG snapshot and
reproducibility assertions. Dense synthetic graphs and small exhaustive search
equivalence tests remain PR-blocking; size-related names alone do not justify
deferral. Existing ignored calibration, diagnostics and manual benchmarks remain
opt-in and are not selected by the regression filter.

The subnet route-shape guard compares against a reference using the same visible
border frames, relationship stroke and soft occlusion as the renderer. An
obstacle-only shortest route can run along a card boundary even when its centerline
does not enter the card. If that route violates minimum border clearance, each
pair of additional bends in the border-aware reference permits one short offset.
Otherwise the offset allowance stays zero. Final routes must preserve the
reference's minimum-clearance safety and cannot add bends, reversals, backtracking
or S-jogs. A focused guard test rejects boundary-hugging shortcuts and extra doglegs.

Reproduce all deferred checks with the pinned Docker toolchain:

```sh
docker compose exec -T dev cargo test --locked --all-features --lib deferred_regression_ -- --ignored --test-threads=2 --nocapture
```

For one failure, replace the prefix with its full test name and add `--exact`
after `--`. Compile separately with `cargo test --locked --all-features --lib
--no-run`. Two test threads limit concurrent expensive routing work.
The workflow retains a failing job status, console diagnostics and a
`routing-regressions-<SHA>` log artifact for 14 days, including assertion failures.
Inspect the failed test and revision in Actions, reproduce at that SHA, and fix
the regression in a PR; use normal Actions failure notifications to track these
non-gating failures. No automatic retry or success suppression hides failures.

### Local timing

The [#253 comparison](https://github.com/ritz-p/planorama/pull/410) measured local
standard-suite harness time falling from 119.49 to 70.98 seconds after deferring
three repeated large-fixture tests. Their separate two-thread run took 140.201
seconds. This moves work off PRs; it does not eliminate it or establish hosted
end-to-end savings.

## Test runner evaluation (#254)

Keep `cargo test` as the CI runner. Nextest did not improve the Standard suite
at either tested concurrency, so adding installation and configuration is not
justified by this experiment. Re-evaluate if the suite distribution changes.

The 2026-10-15 two-repeat comparison found nextest slower for standard by 3.6%
at two threads and 1.4% at eight. Large differed by less than one second.
[PR #411](https://github.com/ritz-p/planorama/pull/411) records the decision;
[the original measurements](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/ci.md#test-runner-evaluation-254)
retain per-run timings, CPU, inventories and failure probes. These historical
results do not predict the current suite's runtime.

The comparison uses the post-#253 test selection at `efb7de9`, Rust 1.85.1,
the optimized test profile and cargo-nextest 0.9.148 in Docker Desktop Linux
on an AMD Ryzen 9 5950X with 32 visible logical CPUs and no container CPU quota.
The experiment varies test concurrency, not the VM's CPU allocation. Rust sources and fixtures stay identical across
runners. These historical observations used the former Python tool; they are not
measurements from its JavaScript replacement. The
[comparison script](../tests/support/compare_test_runners.cjs) reuses
the CI suite selector, verifies names and ignored attributes against each
libtest binary, and rejects unexpected nextest filters. It compiles before
timing, then alternates runner order and reverses concurrency order on the second
repeat. JSON records command wall time, child-process CPU time, revision,
tool versions, inventories and per-run logs. Existing ignored benchmarks and
deferred regressions are excluded from both runners.

The JavaScript tool uses Node's monotonic `performance.now()` for elapsed time
and GNU `time` for the command's user plus system CPU time, including descendants
(CPU precision: 0.01 seconds). Both stdout and stderr go to the per-run log;
failed runs are recorded before the tool exits unsuccessfully. This development
tool uses only Node built-ins and the Docker image's `time` package.
After downloading nextest as below, run
`docker compose exec -T -e NEXTEST_BIN=/tmp/planorama-nextest dev node --test tests/support/tooling.test.cjs`
to exercise both runners against a temporary dependency-free crate, including
ignored inventory, execution order and failed-run logs. These probes validate the
tool and do not replace repository performance measurements.

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

## On-demand spacing calibration

Choose **Actions → Spacing budget calibration → Run workflow** and select a branch
or tag. The default source is its immutable SHA captured at dispatch. To run a
specific repository commit, supply the optional full, lowercase 40-character
`source_sha`; short SHAs and branch names in that input are rejected.

```sh
gh workflow run spacing_calibration.yml --ref main
gh workflow run spacing_calibration.yml --ref main -f source_sha="$(git rev-parse HEAD)"
```

The selected workflow definition must already exist on the default branch for
dispatch to be available. Checkout verifies the intended SHA, disables persisted
credentials and has only `contents: read`. The revision's `rust-toolchain.toml`
pins Rust (currently 1.85.1); Cargo uses locked dependencies. Compilation and the
unchanged ignored `budget_calibration` test run in separate steps. It writes only
the requested report directory and does not regenerate maintained examples or
update PR branches. The `spacing-calibration-<SHA>` artifact is retained for 14
days and contains `spacing-calibration.json` and nine comparison SVGs; missing
output fails the upload. This compares previous numerical budgets, defaults and
doubled candidates on large, dense-corridor and constrained fixtures. See
[the experiment](../examples/terraform-large/spacing-calibration.md).

The workflow is independent of the required test aggregate and generated-head
validation dispatch/reporter. A future schedule or post-merge trigger can use
the same job and captured `github.sha` without adding a dependency to **Tests**.

### Calibration split timing (#406)

[PR #408](https://github.com/ritz-p/planorama/pull/408) records identical-source,
warm-dependency Docker measurements: standard command time changed from 242.355
to 166.437 seconds, with calibration alone costing 74.663 seconds. The single
comparison excludes hosted setup, queue, transfers and the large suite; a manual
calibration run still pays its own setup, compilation and artifact costs.

## When generation runs

The [combined multi-account example](../examples/multi-account/README.md) keeps
its fast input/topology regression inside `large_example`. Full SVG/HTML snapshot
rendering and layout measurements are opt-in ignored tests run with `--release`;
the ordinary suite does not acquire their substantial routing cost. Maintained
SVG regeneration builds the release binary for this example. HTML remains an
explicitly regenerated snapshot outside automatic SVG-only writeback.

Run `sh scripts/update-examples.sh` inside Docker whenever rendering behavior,
generation inputs, fixtures or generation scripts change. Input, fixture and
generation-script changes require regeneration even when no diagram difference
is expected; inspect the generated diff to verify that expectation.
Documentation-only and test-only changes are exempt only when they change none
of those categories. Relevant locations include:

- Renderer and build inputs: `src/**`, `build.rs`, `assets/**`, `Cargo.toml`,
  `Cargo.lock`, `rust-toolchain.toml` and `.cargo/**`.
- Committed JSON inputs: `tests/fixtures/**/*.json` and `examples/**/*.json`.
- Snapshot fixture builders: `tests/unit/layout.rs`,
  `tests/unit/layout/containers.rs` and `tests/unit/layout/ports.rs`.
- Generation configuration: `scripts/generated-svgs.txt`,
  `scripts/update-examples.sh` and `.github/workflows/update_examples.yml`.
- Combined-example sources: `examples/generate_multi_fixture.rs`,
  `examples/multi-account/generator.rs`, `examples/terraform-large/generator.rs`
  and `scripts/render-multi-account.sh`.
- Maintained SVGs listed in the manifest, so manual edits to generated output are
  checked against the current renderer.

PR creation and synchronization never start full regeneration. There is no
changed-file matcher or SVG-current certificate. Use the canonical manifest and
script locally:

```sh
docker compose exec -T dev sh scripts/update-examples.sh
git diff --stat
git diff -- examples
```

Inspect the affected diagrams and commit the necessary outputs with the source
change. When adding generation inputs, update the script and manifest as needed.
The local script also refreshes `examples/coverage.md` alongside the basic SVG.
The manual workflow's SVG writeback remains limited to the SVG manifest; commit
Markdown report changes locally.
For recovery, choose **Actions → Update SVG examples → Run workflow**, select
`main` for the workflow definition, and supply the open PR number, or run:

```sh
gh workflow run update_examples.yml --ref main -f pull_request=123
```

The workflow captures that PR's latest head, including for forks, and checks out
the exact SHA regardless of changed paths. The selected workflow ref is not the
generation source. Rerunning a job may capture a newer PR head; consult its summary.

### Ordinary snapshot coverage

Ordinary unit/integration tests compare these maintained SVGs with current
rendering: `association`, `bundling`, `checks`, `components`, `containment`,
`data`, `data-icons`, `service-names`, `dense-architecture`, `diagram`, `hierarchy-routing`,
`multi-container`, `multi-plan`, `port-collisions`, `regions`, `regional-states`,
`relationship-index`, `relationship-index-wide`, `routes`, `spanning-dense`,
`subnet-groups`, `terraform`, `terraform-large/diagram`, `vpc-workloads`,
`workload-security-groups`, `aurora`, and `elasticache` (paths under `examples/`, with `.svg`).

Other maintained SVGs are listed in `scripts/generated-svgs.txt`. Their related behavior has focused tests, but ordinary tests do not reproduce these
committed SVGs byte for byte. The multi-account input/topology test reads the
committed SVG; its full rendering comparison is ignored and opt-in. These outputs
can become stale while ordinary checks pass, so local generation and visual diff
review remain necessary. Rust tests, formatting, Clippy, representative HTML
snapshot tests and the existing Docker browser test remain unchanged; none claims
to validate all maintained SVGs on ordinary PRs.

## Generation and review

1. Capture the open PR's exact head SHA and check out that revision.
2. Run `sh scripts/update-examples.sh` using the pinned Rust toolchain and locked
   dependencies. [`scripts/generated-svgs.txt`](../scripts/generated-svgs.txt) is
   the shared generation list and writeback allowlist.
3. Upload an artifact containing the complete SVG output under `svgs/examples/`,
   an `updates.patch` file, and revision-bound `updates.json` metadata.
4. For a writable branch in this repository, create one
   `chore: regenerate SVG examples [skip ci]` commit containing only changed SVGs.
   Register each applicable validation context as pending on its SHA before
   publishing it on the PR branch.
5. Atomically update the PR branch to that commit, then explicitly start the
   selected validation workflows for the resulting PR head.
   The workflow summary links each run. The auxiliary **SVG validation dispatch**
   status records whether all required dispatch requests were verified.
6. Publish the actual validation results as commit statuses on that same head.
   A successful dispatch status means the requests succeeded; the validation
   statuses determine whether the checks passed.

The generation command covers JSON-based examples, multi-state/cross-state
examples, and the two test-built snapshots `hierarchy-routing.svg` and
`port-collisions.svg`. It selects those two existing fixture tests with their
snapshot update flags, rather than running the entire test suite. Missing or
renamed fixture tests fail generation instead of silently matching zero tests.
Generation does not wait for the ordinary snapshot comparison checks to pass.

Each generated file must exist and be a regular UTF-8 file. A generation failure
does not publish partial changes. No-diff output creates no commit. Adding a new
maintained example requires adding its output to the manifest and, if it does not
follow the existing fixture naming convention, its input command to the script.
Source files, plan JSON, documentation and original icon assets are not writeback
targets. Only committed fixtures are used; no Terraform apply or cloud account is
needed.

Artifacts are retained for 14 days. On the matching source revision, a downloaded
patch can also be applied manually:

```sh
git apply --check /path/to/updates.patch
git apply /path/to/updates.patch
git add -- examples
git commit -m "chore: regenerate SVG examples"
git push
```

Check the staged changes before committing if the working tree already contains
other edits. Local regeneration remains available through the
[same Docker/PowerShell entry points](../CONTRIBUTING.md#svg-の更新).

## Branch updates and permissions

Generation runs with read-only repository permissions. A separate fresh runner
performs writeback, with `contents: write`, `actions: write`, `statuses: write` and
`pull-requests: read`. Preparation, packaging and writeback use
`actions/github-script`. The write-enabled job reads the output allowlist again
from the captured source commit through the
GitHub API and handles the artifact as data. It does not check out or build the PR
source, run downloaded scripts or apply an artifact patch.

GitHub creates a tree and commit containing the declared SVG changes. The branch
update uses GraphQL `updateRefs` with the captured `beforeOid` and `force: false`.
If the head changes before this atomic update, including a branch rewind, the
update is rejected. Discard the old result and regenerate locally or dispatch
again for the newer head. The SVG workflow's per-PR concurrency does not cancel a
running writeback between its commit and validation dispatch.

Fork PRs keep the patch/artifact flow and do not enter the write-enabled job.
Protected branches, read-only tokens and repository policy restrictions also fall
back to the artifact. Generation and writeback do not use `pull_request_target` or
require a personal token for the ordinary same-repository case.

## Validation on the generated head

The standard `GITHUB_TOKEN` is used for both writeback and explicit
`workflow_dispatch` requests. The bot commit's `[skip ci]` directive suppresses
duplicate push/PR-event runs, including the token-generated PR runs that would
otherwise require approval. Explicit dispatch is unaffected by that directive and
starts validation on the generated head without an additional approval in the
ordinary same-repository flow. The directive does not cancel runs already created
for an earlier commit; dispatches remain independent of PR-event concurrency.

Formatting, Clippy and Tests are dispatched after an SVG commit. MSRV is included
when the full PR file list touches its existing toolchain/Cargo/Docker inputs,
MSRV workflow or shared validation checkout helper. **Workflow tests** is included
for changes under `.github/**`, `tests/ci/**`, `tests/layout-quality/**`, or to
`scripts/layout-quality.cjs`. This validation selection
examines the whole PR, including changes before the generated commit, independently
of local generation decisions.

Dispatch specifies the PR branch, PR number and exact expected head SHA. Its run
name is `Validate PR #<number> at <full head SHA>`, and the returned run ID is checked
against that SHA, event and workflow. Six bounded reads allow the run identity to
appear; persistent mismatches fail with the differing fields in the log.
When these PR inputs are provided, each validation workflow also checks its own
run/checkout SHA, fetches the PR merge
ref, verifies its parents against the expected head and current base, and checks
the PR metadata again. A stacked PR may use its predecessor's generated merge
as the first parent. This is accepted only when exactly one open same-repository
predecessor PR identifies that merge and its head equals the advertised base.
The generated merge's parents must match the predecessor's current base and head,
and its tree must equal the advertised base tree. Both PRs' metadata must remain current throughout these
checks. This tests the PR/base merge while binding the run to the
generated head. Short, bounded retries allow GitHub's PR metadata and merge ref to
catch up after the bot commit. A stale dispatch, a conflicting PR or a merge ref
that has not caught up fails explicitly instead of testing a different revision.

### PR results and Actions jobs

GitHub does not use jobs from `workflow_dispatch` runs as PR checks or required
status checks. Explicit dispatch alone therefore does not restore the checks
suppressed by `[skip ci]`. **Report PR validation** reads the validation runs and
uses the Commit Status API to publish their results on the generated PR head.
Both writeback and reporting use `GITHUB_TOKEN`; no additional credentials or
workflow approval are needed in the ordinary same-repository flow.

| Validation | Ordinary PR job check | Dispatched job in Actions | Reported PR commit status |
| --- | --- | --- | --- |
| Formatting | `Formatting` | `Dispatched Formatting` | `Formatting` |
| Clippy | `Clippy` | `Dispatched Clippy` | `Clippy` |
| Tests | `Tests` | `Dispatched Tests` | `Tests` |
| MSRV, when applicable | `MSRV` | `Dispatched MSRV` | `MSRV` |
| Workflow tests, when applicable | `Workflow tests` | `Dispatched Workflow tests` | `Workflow tests` |

The Tests row names the aggregate check. Parallel suite jobs use
`Test suite (standard)` / `Test suite (large)` for PR events and
`Dispatched Test suite (standard)` / `Dispatched Test suite (large)` for dispatch.
The reporter requires both dispatched suites' merge-checkout, compilation and test
steps to succeed, together with the aggregate's **Check test suites** step. A
failed, cancelled, skipped, missing or duplicated suite cannot publish success.
The pre-split serial `Dispatched Tests` shape remains supported for older branches
and runs; a partially present split execution cannot fall back to that shape.

The different dispatched job names avoid a same-name job check and commit status
on the generated head. Writeback initializes the applicable validation contexts
as pending before publishing a new SVG commit on the PR branch. The workflow list
selected for initialization is reused for dispatch, so both operations cover the
same checks. After publication, only the reporter updates those contexts and links
each result to its validation run. A no-diff dispatch repair does not initialize
them again; the reporter handles subsequent attempts and their results.

Writeback also maintains **SVG validation dispatch**, whose `pending`, `error` and
`success` states describe dispatch progress. Its success does not claim that the
tests passed. If the reporter cannot run or fails before publishing a result, the
initial validation statuses remain pending even when dispatch succeeded.

The reporter listens for `workflow_run` requests, starts and completions. It
executes reporting code from its trusted `github.workflow_sha`, without checking
out or executing PR code. Its repository write access is limited to
`statuses: write`. It reads GitHub's run/job and PR metadata, verifies the
allowed workflow path and ID, the PR number and expected SHA, and every expected
dispatched job's required steps. Only the latest eligible run/attempt
for the exact current generated PR head can update its status. Earlier heads,
old attempts and delayed events cannot supply a successful result for newer code.
Queued or running validation is reported as pending; success requires its expected
steps to pass. Missing or skipped validation steps do not count as success.
Runs without the bound PR inputs, including ordinary manual runs on `main`, are
not reported as validation of the generated head.

Required status checks may use the stable names `Formatting`, `Clippy`, `Tests`,
`MSRV` and `Workflow tests`; the workflow does not change repository rules. Ensure
a required check runs for every PR covered by the rule: normal workflows use path
filters, and MSRV and Workflow tests are dispatched only for their relevant inputs.
Requiring a conditionally absent check leaves the PR waiting. Select the validation
names, not `Dispatched ...`, the reporter job or **SVG validation dispatch**. This
mechanism supplies required status checks; it does not establish that a rule requiring a particular
workflow execution has been satisfied.

Do not require **Generate SVG examples** or **Commit SVG updates and start
validation**: manual generation is optional and creates no pending status when
not dispatched.

See GitHub's [required-check rules and event restrictions](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)
and [Commit Status API](https://docs.github.com/en/rest/commits/statuses).

### Recovery

| Situation | Action |
| --- | --- |
| SVG generation fails or snapshots are missing/stale | Fix the problem, regenerate locally and commit the outputs, or dispatch **Update SVG examples** with the PR number after pushing the fix. A push alone does not regenerate. |
| SVG commit exists but validation dispatch failed | Run **Update SVG examples** manually with the PR number. A no-diff result on the unchanged generated head repairs dispatch without another commit. |
| A validation status fails | Open its linked validation workflow run. Fix code failures and push; retry that run for a transient or merge-ref timing failure. |
| Validation stays pending or reporting failed | Inspect **Report PR validation** and retry its failed run. It rereads the current run and PR before reporting, so a reporting API failure does not require rebuilding or regenerating SVGs. |
| Another push supersedes the generated head | Follow validation for the new head; earlier results are not transferred to it. |

The no-diff repair recognizes generated-commit markers with or without the newer
`[skip ci]` suffix. It preserves individual validation statuses until the reporter
processes a new attempt, avoiding a race between dispatch initialization and
reporting. A manual validation run without both PR inputs remains an
ordinary branch validation; use **Update SVG examples** with the PR number to repair the
validation flow for the generated head.

If dispatched runs exist but no reporter run appears, inspect the reporter trigger
on the default branch. Editing its definition only in a PR cannot activate that
change. Ordinary PR-event checks on a subsequent commit remain separate validation
of that commit; earlier pending statuses do not represent its result.

### Reporter verification

Manual dispatch requires workflow definitions on the default branch. The reporter
uses its trusted default-branch implementation, so a PR-only reporter change cannot
demonstrate the complete writeback/reporting cycle. Older PR branches must include
compatible generation tools and validation definitions.

The separate [Workflow tests](../.github/workflows/workflow_tests.yml) workflow
runs the `Lint workflows` step with pinned actionlint and the
`Test validation reporting` step with Node's built-in test runner for
`tests/ci/**`, including suite-selection and multi-job reporting regressions. Rust
builds, tests and SVG generation remain in their existing CI workflows.
After changing the reporter and merging it to the default branch, verify that the
SVG-only bot commit receives the expected commit statuses, each linked to the
matching SHA's validation run. Check failure and retry reporting as well as
successful completion, without treating dispatch success as validation success.

## Validation after merging

Tests and Clippy also run when a PR with relevant changes is merged into `main`.
Their `pull_request_target` trigger accepts only the `closed` activity for `main`,
and the jobs require `merged == true`. They keep read-only repository permissions
and the default checkout of the event's default-branch commit. They do not check
out the PR head in this post-merge path. These runs populate the default-branch
dependency caches for later PRs.

GitHub's skip directives apply to `push` and `pull_request`, so a bot commit's
`[skip ci]` can also suppress a push workflow if retained in a squash-merge
message. The merged-PR trigger is unaffected and replaces the previous `main`
push trigger, avoiding duplicate post-merge runs. A direct push to `main` requires
manually dispatching Tests and Clippy on `main`, leaving the optional PR inputs
empty. Formatting and MSRV retain their existing PR and manual-dispatch triggers.

References: [workflow triggers and GITHUB_TOKEN](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow),
[skip directives](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs),
[workflow-run reporting](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run),
[merged-PR events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request_target-workflow-when-a-pull-request-merges),
[workflow dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event),
[atomic Git ref updates](https://docs.github.com/en/graphql/reference/git#updaterefs).

## Browser Wasm (#432)

The `Browser Wasm` workflow builds the native CLI and Wasm library in the pinned
Docker environment on pull requests, manual dispatch and cache-input pushes to
`main`. It checks exact output
parity, offline generation, isolated previews, downloads and cancellation using
Chromium. It also builds and checks the standalone artwork-free engine archive,
including native output parity, JS/Wasm artwork exclusion and reproducible
packaging. The no-default-feature build runs the `artwork_free::` library tests
and CLI help checks. The full library suite is available through the manual
`full_library` input. Both distributions run memory-budget tests with small fixtures at every
configured maximum, binary import/capability checks and tiny-memory allocation
failure/trap recovery. Multi-GiB growth and pointer tests remain opt-in; see the
[memory contract](browser.md#per-generation-memory-selection). Artwork-exclusion tests also run with all features enabled. The manual
**Wasm engine release** workflow distributes this engine separately from the UI;
see [browser build, distribution and verification](browser.md). Unit tests now run
from the library target (`--lib`); suite discovery includes that target automatically.

### Browser Wasm caches and measurements (#674)

The required `browser` job still runs on every PR and manual dispatch, with the
same 20-minute timeout. PR coverage has no path-based skip or cross-job binary transfer.
Pushes to `main` that change cache inputs run the same validation to seed trusted
caches for later PRs; manual dispatch on `main` can also populate an empty cache.
Default and `--no-default-features` build commands both run even though the
current default feature list is empty: Cargo exposes the `default` feature to
the compiler. They share `target/shared` so Cargo can reuse common dependencies
while tracking each feature configuration. Browser validation completes before
the standalone build replaces the native reference executable. Both binding,
memory and distribution paths run on every invocation.

Buildx loads the pinned Dockerfile image as `planorama-wasm-ci:local`, restoring
and exporting GitHub Actions BuildKit layers under `browser-wasm`. Tool layers
copy only their installation scripts; ordinary source edits do not invalidate
Rust, Terraform, Chromium or wasm-bindgen installation. `compose.wasm-ci.yaml`
overrides the registry/git/target named volumes with `.wasm-ci` bind mounts so
the restored host files are actually visible inside the container. The local
Compose setup retains its named volumes. Container-created cache files are
returned to the runner's ownership before upload.

Cargo caches separate native and Wasm targets; the native cache includes release
and test profile directories shared by both feature configurations. Together with
the registry/git cache, this uses three archives instead of five. Keys include
runner OS/architecture, Docker/Compose, toolchain, manifests/lock, Cargo config,
build script, assets, scripts and workflow. A changed key starts empty, without
a broad fallback. Workspace products and CLI test binaries are removed before
save; bindings and distribution archives are always generated from freshly built
workspace code. Only successful runs save Cargo caches. Immutable dependency
keys avoid uploading the same dependencies on every source edit.

Native compilation retains mold; Wasm retains `wasm-ld` and its memory flags.
The existing container sccache wrapper can reuse eligible compilations within
the job, with incremental compilation disabled and a 2 GiB local limit. Its
cache is not separately transferred: the [compiler-cache measurements](build-performance.md#compiler-cache)
show a small Wasm benefit before transfer overhead, while the Cargo caches
already avoid those compiler requests. No host wrapper settings are assumed.

All jobs use `contents: read` and checkout without persisted credentials.
GitHub's [PR cache scope](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache) isolates untrusted writes from the base branch and
other PRs; the workflow never uses `pull_request_target`. No secrets or external
cache service are required. A miss rebuilds everything. If a restored cache
causes failures, delete the affected `wasm-ci-v2-*` entry and rerun; bump the
prefix for a full Cargo reset. Delete the `browser-wasm` BuildKit cache to reset
environment layers. Cache hits never bypass build or validation commands.

`sh scripts/wasm-ci.sh` runs the full sequence inside Docker and writes
`output/wasm-ci/{timings,bindings}.tsv` (phase, wall seconds, user seconds,
system seconds, peak RSS KiB), environment, cache sizes and sccache statistics.
`WASM_CI_REPORT_DIR` selects another repository-relative report directory.
`browser.tsv` records non-overlapping browser test sections in wall seconds,
including each service fixture/format, incomplete inputs, large-input UI
progress/cancellation/retry, Wasm completion, and native comparison separately.
It contains fixed test labels and timings, not input or generated content.
`selection.txt` records whether the complete library suite was selected.
The seven-day `wasm-ci-<sha>-<attempt>` artifact carries these records. Action
step timestamps include environment import/export, cache restore/save and
artifact upload overhead; use the complete job duration when comparing runs.
Native reference reuse is process-local, bounded to 64 MiB of UTF-8 output and keyed by input
bytes, the full argument vector/options and executable SHA-256. Failures are
never cached. Named/file-based cases remain uncached, and every Wasm case runs.

Invalid memory-option values are checked once at the default memory budget;
each configured maximum retains generation, output equality, cancellation and
budget-mismatch checks. Extracted-engine parity retains all four fixtures in SVG
and HTML; omitted/explicit icon-mode equivalence and unsupported icon-mode
rejection use `viewer-plan` as the representative fixture.

### Browser Wasm comparison, 2026-10-26

[PR #675](https://github.com/ritz-p/planorama/pull/675) records the original cache
comparison and hosted phase timings. Complete job time was 707 seconds before,
885 seconds with initial cold caches, and 585 seconds with fully warm caches.
Initial BuildKit export added cost; warm reuse reduced it. These single-run
measurements predate test consolidation and do not establish current latency.
[The original local measurements](https://github.com/ritz-p/planorama/blob/8fb4aef052f7d941b0a5da6b08a4fc610661a504/docs/ci.md#browser-wasm-comparison-2026-10-26)
retain environment, phase and cache-size details.

### Browser Wasm test scope audit (#885)

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

The default selection no longer repeats 898 of the 900 passing library tests
under no-default-features (eight ignored tests were not running before either).
No browser assertion or fixture/format pair is removed. A future feature-specific
implementation outside `artwork_free` could therefore escape the focused native
suite; feature changes must revisit this audit. Manual `full_library: true`, or
`WASM_CI_FULL_LIB=1 sh scripts/wasm-ci.sh` in Docker, restores the complete suite
for investigation. It is not a scheduled replacement or a claim of identical
coverage for arbitrary future feature combinations.
The separate Wasm engine release workflow still runs the complete library suite.

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
