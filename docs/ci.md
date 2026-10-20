# SVG generation and PR validation

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
build inputs. It compares deterministic Markdown using one unit test; it performs
no SVG generation or routing. The same test belongs to the standard Rust suite.
See [regeneration instructions](../CONTRIBUTING.md#リソース対応一覧の更新).

## Toolchain and dependency cache

Local setup and check commands live in [CONTRIBUTING](../CONTRIBUTING.md).
Ordinary Formatting, Tests and Clippy use the pinned `rust-toolchain.toml` on
Ubuntu runners. Tests and Clippy cache Cargo registry/git data and dependency
build products with `Swatinem/rust-cache`; workspace crates are rebuilt each run.
The `v1-rust-dependencies` prefix excludes older workspace-product caches.
Keys include the runner/compiler environment, Cargo manifests and lockfile,
toolchain/Cargo configuration, build-related environment and the `build.rs` /
`assets/**` hash. Test suites share the `-tests` key; Clippy uses a separate key.
Cargo still checks rebuild requirements, and cache misses do not reduce coverage.

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
the `build.rs`/`assets/**` hash followed by `-tests`, matching the former serial
job's key. Clippy retains its separate cache. The pinned compiler, Cargo configuration,
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

On 2026-10-12, the [serial baseline](https://github.com/ritz-p/planorama/actions/runs/38224372411)
at `754ffd2` and [parallel run](https://github.com/ritz-p/planorama/actions/runs/38225451104)
at `e365bd2` used identical Rust sources, fixtures, Rust 1.85.1 and Ubuntu runner
image `20261004.327.1`. Both parallel jobs restored the exact same 17,658,308-byte
dependency cache as the serial job. Each arrangement was measured once; these are
observations rather than a latency guarantee or a billed-cost estimate.

| Measurement | Serial | Parallel | Change |
| --- | ---: | ---: | ---: |
| First job start to final job completion | 170 s | 141 s | -29 s (-17.1%) |
| Sum of all test-workflow job durations | 170 s | 201 s | +31 s (+18.2%) |
| Workflow creation to final update | 204 s | 184 s | -20 s (-9.8%) |
| Queue/provisioning before the first job | 33 s | 42 s | +9 s |
| Passed / ignored / failed tests | 506 / 1 / 0 | 506 / 1 / 0 | Unchanged |

| Parallel job | Total job time | Compile tests | Run tests |
| --- | ---: | ---: | ---: |
| [standard](https://github.com/ritz-p/planorama/actions/runs/38225451104/job/114731950802) | 137 s | 77 s | 48 s |
| [large](https://github.com/ritz-p/planorama/actions/runs/38225451104/job/114731950398) | 62 s | 29 s | 19 s |
| [Tests aggregate](https://github.com/ritz-p/planorama/actions/runs/38225451104/job/114732772522) | 2 s | — | — |
| Total runner time | 201 s | 106 s | 67 s |

Both suites started together and overlapped for 62 seconds. The large suite
finished while the standard suite was still compiling; their test-execution steps
did not overlap in this run. The serial compile/test steps took 82/70 seconds.
Its ordinary integration targets accounted for only 1.40 seconds, supporting the
choice to keep those targets with unit tests instead of adding a third runner.
Parallelism saves waiting time at the documented cost of additional workspace
compilation and runner setup. Workflow, Clippy and formatting validation outside
**Cargo test** are excluded from both sides of this comparison.

## Layout quality comparison (#390)

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

Measured once per selection on 2026-10-15 in Docker Desktop Linux, Rust 1.85.1,
32 visible logical CPUs, default libtest concurrency and the optimized test
profile. Baseline `25d3927` used `cargo test --locked --all-targets`; after adding
only ignore attributes and name prefixes, the two existing suite selectors ran
with `--all-features` (this package declares no features). Sources, assertions,
fixtures and dependencies were otherwise identical. Compilation was completed
before the after measurements; no other Rust tests ran concurrently. Host load
was not controlled, and Node workflow tests overlapped part of the after run.

| Harness execution time, excluding compilation/startup | Before | After |
| --- | ---: | ---: |
| Unit binary | 115.89 s | 67.12 s |
| Standard suite, sum of harness times | 119.49 s | 70.98 s |
| Large suite | 71.36 s | 75.93 s |
| Ordinary passed / ignored / failed | 630 / 5 / 0 | 627 / 8 / 0 |

Standard harness time fell 40.6%; the unchanged large suite is now the longer
test phase. The complete after Standard command took 72.150 s. These local
observations exclude hosted checkout, queueing, cache and job startup, so they
do not establish an end-to-end CI speedup. Deferred execution retains the three
remaining assertion sets without putting them back on the PR critical path.
The deferred command passed all three tests in 140.201 s (139.51 s in the
harness), with its explicit two-thread limit; it is additional post-merge/nightly
compute, not work that disappears.

## Test runner evaluation (#254)

Keep `cargo test` as the CI runner. Nextest did not improve the Standard suite
at either tested concurrency, so adding installation and configuration is not
justified by this experiment. Re-evaluate if the suite distribution changes.

Measured on 2026-10-15. Each cell contains the first and second command wall times:

| Suite | Test threads | Cargo test | Nextest |
| --- | ---: | ---: | ---: |
| Standard | 2 | 179.839 / 182.069 s | 188.001 / 186.936 s |
| Standard | 8 | 89.290 / 89.395 s | 89.542 / 91.723 s |
| Large | 2 | 77.464 / 76.551 s | 76.745 / 76.313 s |

For Standard, nextest was 3.6% slower at two threads and 1.4% slower at eight
threads on the two-run means. Mean child CPU time was 299.8 / 313.2 seconds
(Cargo / nextest) at two threads and 306.3 / 320.6 seconds at eight. Higher
concurrency reduced elapsed time while slightly increasing CPU work for both
runners; nextest used more CPU without reducing wall time. Every Standard run
passed the same 624 tests and skipped the same seven ignored tests.
Every Large run passed three tests and skipped the same manual snapshot test.
Its mean difference was only 0.479 seconds (0.6% in nextest's favor), with
approximately one CPU core occupied throughout. A single large rendering test
dominates that job, limiting scheduling gains. Keep the existing two independent
jobs: combining them into one nextest command would measure a different CI
architecture. These observations do not demonstrate a consistent reduction in
the current parallel workflow's critical path.

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

Both runners reuse the same Cargo artifacts and dependency volumes; warm compile
checks take approximately 0.7 seconds without rebuilding workspace code. This
verifies local build-cache compatibility, not hosted cache restore/install cost.
The existing Actions dependency cache can remain unchanged, but nextest would
require a separately installed binary. Download, cold compilation, checkout,
queueing and cache transfer are excluded from the timings. Host load is not
controlled; no other Rust build or test runs alongside the measured commands.

After timing, a temporary dependency-free crate containing one deliberate
assertion failure and one ignored passing test checked failure handling. Both
runners displayed the failed test name, assertion message, source file/line and
captured stdout. Cargo exited 101 and nextest exited 100; both are failures.
Both skipped the ignored test normally and passed it with `-- --ignored`,
returning zero. The probe did not run repository benchmarks or alter fixtures.
Nextest's failure output and ignored-test compatibility are adequate; measured
performance, rather than compatibility, is the reason for retaining Cargo.

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

Measured on 2026-10-15 in the existing Docker Desktop Linux development container,
Rust/Cargo 1.85.1, optimized test profile. Both observations used identical Rust,
fixtures, assets, Cargo metadata/lockfile and toolchain from `c5e09b3`; #405/#406
only change workflows, their Node tests and documentation. Before each observation,
`cargo clean -p planorama` removed workspace build products while retaining the
same warm dependency cache. Both compile logs rebuilt only `planorama`. No other
Rust build or test ran concurrently; host/background load was not controlled.

Each phase was timed with Python's monotonic clock around its complete process.
The before sequence ran `node .github/scripts/test-suites.cjs standard compile`,
then `standard run`, then the exact ignored calibration command above with
`PLANORAMA_SPACING_REPORT_DIR=output/spacing-calibration`. The after sequence used
the same first two commands without calibration. Each was measured once.

| Local Standard measurement | Before | After |
| --- | ---: | ---: |
| Compile command wall time | 45.232 s | 44.969 s |
| Ordinary test command wall time | 122.460 s | 121.468 s |
| Additional calibration command | 74.663 s | Not run |
| Sum of command wall times | 242.355 s | 166.437 s |
| Ordinary tests passed / ignored / failed | 627 / 4 / 0 | 627 / 4 / 0 |

Calibration itself reported 73.96 s of test execution and 0.63 s of cached Cargo
build preparation; its remaining process overhead was about 0.07 s. It passed
one test and wrote JSON plus nine SVG comparisons. The observed 75.918 s reduction
includes 1.255 s of unrelated compile/ordinary-test variation; it is not a fixed
speedup. The remaining ordinary tests dominate the after measurement.

These totals measure local command execution, not end-to-end GitHub job time.
Queue/provisioning, checkout, toolchain installation, cache restore/save, artifact
upload, the Large job and the Tests aggregate are excluded. They were not measured
as zero. Local single-container occupied time falls by the same command-time
delta; billed runner savings cannot be inferred. An on-demand run incurs its own
startup, compilation and artifact costs, unlike the measured calibration which
reused Standard's compiled binary. Record those separately when comparing hosted
runs; the large suite and aggregate remain unchanged.

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

Ordinary unit/integration tests compare 27 of the 62 maintained SVGs with current
rendering: `association`, `bundling`, `checks`, `components`, `containment`,
`data`, `data-icons`, `service-names`, `dense-architecture`, `diagram`, `hierarchy-routing`,
`multi-container`, `multi-plan`, `port-collisions`, `regions`, `regional-states`,
`relationship-index`, `relationship-index-wide`, `routes`, `spanning-dense`,
`subnet-groups`, `terraform`, `terraform-large/diagram`, `vpc-workloads`,
`workload-security-groups`, `aurora`, and `elasticache` (paths under `examples/`, with `.svg`).

The other 35 are `ecs-clusters`, `certificate-cards`, `log-groups`, `accounts`, `aws-relationships`, `autoscaling-policy`,
`cross-state`, `logical-actions`, `logical-regions`, `multi-account/diagram`,
`network-scope`, `rds-subnet-group`, `relationship-scopes`, `security-groups`,
`messaging`, `sns-subscriptions`, `event-sources`, `kinesis`, `firehose`, `eventbridge`, `eventbridge-targets`,
`region-overrides`, `relationship-legend-hidden`, `route53`, `acm`, `kms`,
`secrets-manager`, `ssm`, `iam`, `identity-providers`, `efs`, `ebs`, `glue`, `athena`, and `private-ca`.
Related behavior has focused tests, but ordinary tests do not reproduce these
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
Docker environment on pull requests and manual dispatch. It checks exact output
parity, offline generation, isolated previews, downloads and cancellation using
Chromium. See [browser build and verification](browser.md). Unit tests now run
from the library target (`--lib`); suite discovery includes that target automatically.
