# CI validation and recovery

現行の起動条件・判定・復旧手順の正本です。過去の性能比較と採否は [計測履歴](ci-history.md)、ローカル環境は [CONTRIBUTING](../CONTRIBUTING.md) を参照してください。

- [通常テスト](#parallel-rust-test-suites) / [Layout quality](#layout-quality-comparison-390) / [Routing regressions](#deferred-routing-regressions-253)
- [SVG 生成・レビュー](#generation-and-review) / [復旧](#recovery) / [マージ後](#validation-after-merging)
- [Browser Wasm と cache 復旧](#browser-wasm-432) / [リソース一覧](#resource-support-catalog)

Native CLI publication is manual-only (`workflow_dispatch`, `publish: true`,
matching existing version tag); see [release validation and recovery](releases.md).
Ordinary PRs retain lightweight release-control tests. Diagram regeneration and
manual SVG writeback follow [generation and review](#generation-and-review).

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

Formatting, Tests and Clippy use the pinned `rust-toolchain.toml` on Ubuntu.
Tests/Clippy cache registry/git and dependency products with `Swatinem/rust-cache`;
workspace crates rebuild each run. Cache misses never reduce coverage.

| Job | Dependency cache | Compiler/linker |
| --- | --- | --- |
| Both test suites | `v2-mold-2.40.4`; shared `-tests` key | checksum-verified mold 2.40.4 and sccache 0.10.0 |
| Clippy | `v1-rust-dependencies`; separate key | no sccache wrapper |

Keys include runner/compiler environment, manifests/lockfile, toolchain/Cargo
configuration, build environment and `build.rs` / `assets/**` hashes; test keys
also include linker installer/wrapper hashes. Older workspace-product caches are excluded.
Tests and Docker Compose select mold only via
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER`; Wasm keeps its rustflags and
`wasm-ld`, and release/cross/other-architecture linkers are unchanged.

Tests and Browser Wasm disable incremental compilation for sccache. The two test
jobs restore a separate 2 GiB compiler cache: target hits avoid compilation,
sccache handles eligible misses. Only successful runs with new compiler entries
upload it, after stopping the server. Fork/PR cache scopes and permissions are
unchanged. See [measurements and recovery](build-performance.md#compiler-cache).

MSRV runs Docker/Rust 1.85.0 for changes to Cargo.toml, Cargo.lock,
rust-toolchain.toml, Dockerfile, .dockerignore, compose.yaml, its workflow or the
shared validation-checkout helper. See [local setup](../CONTRIBUTING.md) and
[post-merge cache population](#validation-after-merging).

## Cancelling superseded validation

**Cargo fmt**, **Cargo clippy** and **Cargo test** cancel older pending/running
`pull_request` runs for the same workflow and PR. Keys use fixed `cargo-fmt`,
`cargo-clippy`, `cargo-test` prefixes and `pr-<number>`, keeping workflow
renames, unrelated PRs and same-named fork branches independent. Completed results remain.

Every manual `workflow_dispatch` and post-merge `pull_request_target` run instead
uses `run-<run_id>`. A stale SVG-triggered dispatch must fail its exact-head
checkout/reporter checks without cancelling newer PR validation. Repeated manual
runs stay independent; SVG writeback and reporting retain their non-cancelling groups.

See GitHub's [workflow concurrency documentation](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).

## Parallel Rust test suites

**Cargo test** starts two independent jobs with `fail-fast: false`, so a failing
suite does not cancel the other suite's coverage:

| Suite job | Targets |
| --- | --- |
| `Test suite (standard)` | Unit tests, ordinary integration tests, libraries, binaries, examples and benchmarks other than the large integration target. |
| `Test suite (large)` | The `planorama` package's `large_example` integration target. |
| `Tests` | A small aggregate job that succeeds only after both suites succeed. |

The [suite selector](../.github/scripts/test-suites.cjs) reads
`cargo metadata --locked --all-features --no-deps --format-version 1` for default
workspace packages. **Compile tests** adds `--no-run` to the same explicit
`cargo test --locked --all-features` target arguments used by **Run tests**.
Build scripts remain Cargo prerequisites; snapshots stay in their original suites.

All discovered non-build targets join `standard` except `large_example`, including
new integration/library/binary/example/benchmark targets and those with
`test = false`, preserving prior `--all-targets` coverage. Unknown target kinds,
incomplete metadata or missing/renamed/ambiguous `large_example` fail explicitly.
Three ignored large-fixture regressions run in [their own lane](#deferred-routing-regressions-253);
ignored spacing calibration is [manual-only](#on-demand-spacing-calibration).
Neither is a PR gate or dependency of **Tests**.

Both suites use the [shared dependency cache](#toolchain-and-dependency-cache)
but rebuild workspace crates independently, without serial artifact transfer.
Ordinary integration tests stay with unit tests to avoid a third startup/build.
For comparisons, record workflow wall time and summed suite/aggregate job time,
including setup, cache and compilation; separate queue time, record cache hits,
step timings, source/base SHAs and test counts, and keep Rust sources/fixtures
identical. Runner seconds measure compute, not billed price.

### Observed timing and runner cost

2 suite は待ち時間と総 runner 時間のトレードオフです。 [判断・計測記録](ci-history.md#observed-timing-and-runner-cost)。

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

Large-specific thresholds stay in the deferred tests. PRs retain
`large_aws_example_keeps_cross_module_containment_and_multi_subnet_relationships`,
`large_example` SVG snapshots/reproducibility, dense synthetic graphs and small
exhaustive-search equivalence. Size-related names alone do not justify deferral;
ignored calibration, diagnostics and benchmarks remain outside this filter.

The subnet route-shape reference uses the renderer's visible borders, stroke and
soft occlusion. If an obstacle-only shortest route violates border clearance,
each pair of extra bends in the border-aware reference permits one short offset;
otherwise none are allowed. Final routes must preserve reference clearance and
add no bends, reversals, backtracking or S-jogs. A focused test rejects
boundary-hugging shortcuts and extra doglegs.

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

大型回帰は削除せず別 workflow で検証します。 [判断・計測記録](ci-history.md#local-timing)。

## Test runner evaluation (#254)

CI は `cargo test` を継続します。比較で Standard suite の短縮が得られず、suite 分布が変わった場合に再検討します。 [判断・計測記録](ci-history.md#test-runner-evaluation-254)。

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

Calibration is independent of **Tests** and generated-head validation/reporting.

### Calibration split timing (#406)

calibration は通常の Tests から独立した手動実行です。 [判断・計測記録](ci-history.md#calibration-split-timing-406)。

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

Other outputs in `scripts/generated-svgs.txt` have focused behavioral tests,
not byte-for-byte ordinary snapshot coverage. The multi-account topology test
reads its committed SVG; full rendering is ignored/opt-in. These SVGs can be
stale despite passing Rust/formatting/Clippy/HTML/browser checks, so regenerate
and inspect the diff when required.

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

Generation is read-only. A fresh writeback runner uses `actions/github-script`
with `contents: write`, `actions: write`, `statuses: write` and
`pull-requests: read`. It rereads the allowlist from the captured source commit
through GitHub API, treats artifacts as data, and never checks out/builds PR code,
runs downloaded scripts or applies an artifact patch.

The declared SVGs become a Git tree/commit. GraphQL `updateRefs` atomically
updates the branch with captured `beforeOid` and `force: false`; any intervening
head change, including rewind, rejects it. Regenerate for the new head.
Per-PR concurrency does not cancel writeback between commit and validation dispatch.

Forks never enter the write job. Forks, protected branches, read-only tokens and
policy restrictions retain the patch/artifact fallback. Generation/writeback
use neither `pull_request_target` nor a personal token in the ordinary same-repo flow.

## Validation on the generated head

Writeback and explicit `workflow_dispatch` use `GITHUB_TOKEN`.
The bot commit's `[skip ci]` suppresses duplicate push/PR runs (including those
requiring approval), but not dispatch or already-created runs. Same-repository
dispatch needs no additional approval and stays independent of PR concurrency.

Formatting, Clippy and Tests are dispatched after an SVG commit. MSRV is included
when the full PR file list touches its existing toolchain/Cargo/Docker inputs,
MSRV workflow or shared validation checkout helper. **Workflow tests** is included
for changes under `.github/**`, `tests/ci/**`, `tests/layout-quality/**`, or to
`scripts/layout-quality.cjs`. This validation selection
examines the whole PR, including changes before the generated commit, independently
of local generation decisions.

Dispatch binds branch, PR number and exact expected head SHA; the run name is
`Validate PR #<number> at <full head SHA>`. Verify its returned run ID, SHA,
event and workflow, with six bounded identity reads; persistent mismatches log
the differing fields and fail.

PR-bound workflows verify their run/checkout SHA, fetch the PR merge ref, check
its parents against expected head/current base, and recheck PR metadata.
A stacked predecessor's generated merge is accepted as first parent only if:

- Exactly one open same-repository predecessor PR identifies it, with head equal
  to the advertised base.
- Its parents equal that predecessor's current base/head and its tree equals the
  advertised base tree.
- Both PRs' metadata stay current throughout verification.

Bounded retries allow merge refs/metadata to catch up. Stale dispatches, conflicts
and lagging refs fail rather than test a different revision.

### PR results and Actions jobs

Dispatched jobs do not supply PR/required status checks. **Report PR validation**
reads their results and publishes Commit Status API statuses on the generated
head, using `GITHUB_TOKEN` without extra credentials/approval in the ordinary
same-repository flow. Job/status names differ to avoid collisions:

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

Writeback marks selected validation contexts pending before publishing the SVG
commit and reuses that workflow list for dispatch. Thereafter only the reporter
updates them, linking each status to its run. No-diff dispatch repair does not
reinitialize statuses; the reporter processes new attempts.
**SVG validation dispatch** separately reports `pending` / `error` / `success`
for dispatch requests, not test results. If reporting fails, validation stays pending.

The reporter handles `workflow_run` requests, starts and completions using trusted
`github.workflow_sha` code with only `statuses: write`; it never executes PR code.
It validates workflow path/ID, PR number, expected SHA and required job steps.
Only the latest eligible run/attempt for the exact current generated head may
update statuses. Queued/running means pending; missing/skipped steps cannot pass.
Earlier heads, attempts and delayed events cannot validate newer code. Runs
without bound PR inputs, including manual `main` runs, are not reported.

Repository rules may require stable `Formatting`, `Clippy`, `Tests`, `MSRV`
or `Workflow tests` statuses only where those checks run for every covered PR.
Path-filtered/conditional checks can otherwise wait forever. Do not select
`Dispatched ...`, the reporter, **SVG validation dispatch**, **Generate SVG
examples** or **Commit SVG updates and start validation**: manual generation is
optional and creates no pending status unless dispatched. Commit statuses do not
satisfy rules requiring a particular workflow execution. Workflows do not alter
repository rules.

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

Tests/Clippy seed default-branch dependency caches after relevant PRs merge into
`main`: `pull_request_target` accepts only `closed`, jobs require
`merged == true`, permissions remain read-only, and checkout uses the event's
default-branch commit, never the PR head.

This replaces their `main` push trigger to avoid duplicate runs and squash-message
`[skip ci]` suppression (skip directives affect push/PR events, not merged-PR
events). After a direct push, dispatch Tests/Clippy on `main` with PR inputs empty.
Formatting/MSRV retain PR and manual triggers.

References: [workflow triggers and GITHUB_TOKEN](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow),
[skip directives](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs),
[workflow-run reporting](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run),
[merged-PR events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request_target-workflow-when-a-pull-request-merges),
[workflow dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event),
[atomic Git ref updates](https://docs.github.com/en/graphql/reference/git#updaterefs).

## Browser Wasm (#432)

**Browser Wasm** runs the pinned Docker native/Wasm builds on every PR, manual
dispatch and cache-input pushes to `main`. Its required `browser` job has a
20-minute timeout, no PR path skip and no cross-job binary transfer. See
[browser verification](browser-development.md#verification) for UI/parity coverage
and [engine verification](browser-engine.md#per-generation-memory-selection) for
archive, artwork, memory-capability and trap-recovery checks.

Both distributions test small fixtures at every configured memory maximum;
multi-GiB pointer/growth stress remains opt-in. PR no-default-feature native tests
select `artwork_free::` plus CLI help; artwork exclusion also runs with all features.
Manual `full_library` restores the complete `--lib` suite (automatically discovered
by suite selection). **Wasm engine release** validates/distributes the engine
separately from the UI; its complete library suite is retained.

### Browser Wasm caches and measurements (#674)

Cache-input pushes or manual dispatch on `main` seed trusted caches. Default
and `--no-default-features` builds both run because Cargo exposes `default` even
when its list is empty. They share `target/shared` dependencies; browser checks
finish before the standalone build replaces the native reference executable.
Both binding, memory and distribution paths run every time.

Buildx loads `planorama-wasm-ci:local` from the pinned Dockerfile and restores/
exports BuildKit layers under `browser-wasm`. Tool layers copy only installers,
so source edits do not invalidate Rust/Terraform/Chromium/wasm-bindgen installation.
`compose.wasm-ci.yaml` replaces registry/git/target volumes with `.wasm-ci` bind
mounts; ordinary Compose retains named volumes. Cache files return to runner
ownership before upload.

Three Cargo archives cover registry/git, native release/test and Wasm products.
Keys include runner OS/architecture, Docker/Compose, toolchain, manifests/lock,
Cargo config, build script, assets, scripts and workflow. Changed keys start empty
without broad fallback. Only successful runs save dependencies, after removing
workspace products and CLI test binaries; immutable keys avoid repeat uploads.
Bindings/archives always use freshly built workspace code.

Native keeps mold; Wasm keeps `wasm-ld`/memory flags. Container sccache has a 2 GiB
local limit and incremental disabled, with no host-wrapper assumption or separate
transfer: [measured Wasm gains](build-performance.md#compiler-cache) were small
before transfer overhead, and target hits already avoid compilation.

All jobs use `contents: read` and checkout without persisted credentials.
GitHub's [PR cache scope](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache) isolates untrusted writes from the base branch and
other PRs; the workflow never uses `pull_request_target`. No secrets or external
cache service are required. A miss rebuilds everything. If a restored cache
causes failures, delete the affected `wasm-ci-v2-*` entry and rerun; bump the
prefix for a full Cargo reset. Delete the `browser-wasm` BuildKit cache to reset
environment layers. Cache hits never bypass build or validation commands.

`sh scripts/wasm-ci.sh` writes `output/wasm-ci/` (override with repository-relative
`WASM_CI_REPORT_DIR`):

| Record | Contents |
| --- | --- |
| `timings.tsv`, `bindings.tsv` | phase, wall/user/system seconds, peak RSS KiB |
| `browser.tsv` | non-overlapping fixture/format, incomplete-input, large UI progress/cancel/retry, Wasm completion and native-comparison timings |
| `selection.txt` | complete-library selection |
| Environment/cache records | environment, cache sizes, sccache statistics |

The seven-day `wasm-ci-<sha>-<attempt>` artifact excludes input/generated content.
Compare complete job durations too: Action steps include setup, cache transfer and
artifact overhead. Native output reuse is process-local, capped at 64 MiB UTF-8,
keyed by input bytes, full arguments/options and executable SHA-256. Failures and
named/file-based cases are never cached; every Wasm case executes.

Invalid memory-option values are checked once at the default memory budget;
each configured maximum retains generation, output equality, cancellation and
budget-mismatch checks. Extracted-engine parity retains all four fixtures in SVG
and HTML; omitted/explicit icon-mode equivalence and unsupported icon-mode
rejection use `viewer-plan` as the representative fixture.

### Browser Wasm comparison, 2026-10-26

cold/warm cache の差を現在の所要時間として扱わないでください。 [判断・計測記録](ci-history.md#browser-wasm-comparison-2026-10-26)。

### Browser Wasm test scope audit (#885)

PR では `artwork_free::`、両 distribution の build・memory・archive・browser parity を維持します。feature 固有の実装が増えた場合は選択範囲を再検討してください。`full_library: true` または `WASM_CI_FULL_LIB=1 sh scripts/wasm-ci.sh` で完全な library suite を実行できます。Wasm engine release は完全な suite を維持します。 [判断・計測記録](ci-history.md#browser-wasm-test-scope-audit-885)。
