# CI validation and recovery

Local setup and ordinary checks: [CONTRIBUTING](../CONTRIBUTING.md).
Native publication is manual-only with an existing version tag and explicit
publish input; use [release recovery](releases.md).

## Resource support catalog

**Resource support** checks Rust type discovery and managed basic capabilities,
plus inventory extraction/comparison and semantic evidence offline. It does not
download Providers or generate diagrams. Run its commands from
[the workflow](../.github/workflows/resource_support.yml) in Docker.
[Runtime catalogs](resource-support.md) are disposable, on-demand output;
[semantic queries](aws-semantic-audit.md) read the pinned JSON sources directly.

## Toolchain and dependency cache

The toolchain and cache inputs are defined in the workflows, Dockerfile and
rust-toolchain.toml. Missing caches must not reduce coverage. Native test jobs use
mold/sccache; Wasm keeps wasm-ld and release builds retain their own linker.
[Local cache diagnosis/reset](build-performance.md#compiler-cache) is independent
of GitHub caches. Delete an affected sccache cache or bump its prefix to reset it.
Stop the server before saving/removing its directory. PR cache scope must not
permit untrusted writes to the base branch.

## Cancelling superseded validation

Formatting, Clippy and Tests cancel older PR runs for the same workflow/PR.
Manual and post-merge runs remain independent. Stale SVG dispatches must fail
revision checks without cancelling validation of newer heads.

## Parallel Rust test suites

**Tests** succeeds only when both standard and large suites pass. Standard covers
ordinary targets except large_example; large runs that integration target.
The [suite selector](../.github/scripts/test-suites.cjs) is the executable selection
contract. Keep new targets assigned to a suite; do not infer coverage from names.
A failed suite does not cancel the other. CI retains cargo test; revisit runner
choice only when workload distribution warrants a new measurement.

The standard PR critical path has a 600-second design budget. Allow 60 seconds
for runner setup and caches, 240 for compilation, 285 for tests and 15 for
aggregation. Independent format, lint and browser jobs run alongside Rust.
The PR suites build and test in one Cargo invocation per package. Explicit
dispatched validation retains the separate compile step for its status contract.
Inspect the suite summary for Cargo build and test times and the compiler-cache
step for hits and misses.

The weekly or manual **PR timing report** samples completed first-attempt PR
revisions with Rust and browser runs, including format and lint when present.
It measures from the earliest workflow start to the last successful job finish.
Job execution and wait from workflow start are separate fields (dependent jobs
also wait for prerequisites). Its JSON artifact includes per-step timings and
excluded run IDs. Three successive samples over budget raise a warning. Keep
conditional generation, layout, release and offline analysis on their own checks.

## Layout quality comparison (#390)

**Layout quality** runs manually or on PRs with the `layout-quality` label,
including later pushes. Apply it for changes to geometry, routing, text sizing,
rendered nodes/edges, placement, or the quality comparison itself. Diagram-neutral
UI operations, documentation and distribution CI are normally exempt; run if unsure.
It is independent of Tests, SVG regeneration and Routing regressions. Do not make
this conditionally absent job a repository-wide required check.

The [manifest](../tests/layout-quality/manifest.json) is the fixture/argument/budget
authority. It covers small, large, fan-in, cross-state, account configuration and
region inputs; JSON account fixtures do not prove saved-plan account resolution.
The concentrated account stress fixture remains in its separate deferred lane.
[Presentation checks](presentation-quality.md) independently reject text overlap
and overflow in the generated SVGs.

Each baseline binds input hashes, CLI arguments, budgets and the renderer contract.
Changed inputs/options/contracts require review before updating. Unsupported or
incomplete diagnostics are non-comparable, not improvements. Crossings, independent
overlap and parallel deficit are compared in total and per stable edge pair;
border deficit and bends also have per-edge gates. New pairs start at zero.
Allowed increase is absolute + baseline × relative from the manifest. Area and raw
length are informational: length is not shortest-route stretch. Never offset one
fixture/pair's regression with another's improvement.

Run inside Docker:

```sh
cargo build --release --locked --bin planorama
node scripts/layout-quality.cjs check target/release/planorama output/layout-quality
LAYOUT_QUALITY_BINARY=target/release/planorama node --test tests/ci/layout-quality.test.cjs
```

For a PR, verify completion/results on the final pushed commit. For a requested
release, use the publishing SHA; dispatch if that SHA has no successful result and
confirm selected ref and actual checkout SHA match. A release request does not
implicitly authorize creating a tag or publishing.

On failure, inspect each fixture's diff report and SVG. Only an intentional
geometry/input/diagnostic change with a reason in the PR permits baseline/budget
updates. Do not update merely to clear failure:

```sh
node scripts/layout-quality.cjs update target/release/planorama output/layout-quality-update
git diff -- tests/layout-quality
```

Outputs and baselines must use separate directories. An interrupted multi-file
update can be partial: rerun and review all changes. CI never writes baselines.
The workflow retains logs, JSON comparisons, SVGs and presentation reports for
14 days. If execution fails or remains incomplete, report the SHA and reason;
do not merge/release unresolved work or poll indefinitely.

### Reducing a local layout fixture

Use the release binary to inspect `findings` in a layout diagnostics report. Select
the zero-based index of a crossing, independent overlap or parallel proximity.
For example, finding 5 identifies an overlapping edge pair in the AgentCore gateway fixture.

```sh
cargo build --release --locked --bin planorama
node scripts/minimize-layout-fixture.cjs target/release/planorama \
  tests/fixtures/agentcore-gateway-plan.json 5 /tmp/reduced-plan.json
```

The reducer removes groups of root resources and their dependents together. It
checks retained resource identities, providers, scopes and relationship evidence
against the original architecture export. The same edge pair must reproduce at
least its original measurement in complete diagnostics. A final replay precedes
writing the new local file, which contains the retained input values.

Supported inputs are flat, single-state, managed-create fixtures with matching
configuration and change records, explicit providers, local expression references,
and lossless integer literals. Other shapes require a different reduction unit.
The optional last argument bounds deletion trials (default 64, maximum 128).
Each renderer invocation has a 30-second timeout. Use the reported counts and
compare the two input resource lists to inspect the reduction.

## Terraform compatibility (#525)

[Compatibility checks](terraform-compatibility.md) validate committed fixtures on
relevant PRs; fixed-tool capture is manual. The release-branch Compatibility output
job also verifies committed saved plans with external networking disabled, using
the same CLI build. Manual capture uses a local STS fixture. Its
[contract](provider-evidence.md) is distinct from ordinary JSON compatibility.
**Compatibility output** builds once and compares committed matrix SVG/HTML only
on pushes to `release`; its SHA-named artifact records outputs and per-case results.

## Deferred routing regressions (#253)

**Routing regressions** runs after merges to main, nightly and manually. Post-merge
uses the default-branch event commit; manual uses the selected ref. It runs ignored
`deferred_regression_` checks without moving their cost into ordinary PR tests:

```sh
cargo test --locked --all-features --lib deferred_regression_ -- --ignored --test-threads=2 --nocapture
```

For one failure use the full test name plus `--exact`. Inspect the failed revision
and its 14-day `routing-regressions-<SHA>` artifact; reproduce at that SHA and fix
in a PR. These checks preserve large-fixture route shape, stretch and locality
coverage. Ignored benchmarks/calibration are not part of this prefix.

## On-demand spacing calibration

Choose **Actions → Spacing budget calibration → Run workflow**, selecting a ref.
Optional `source_sha` must be a full lowercase commit SHA; checkout verifies it.

```sh
gh workflow run spacing_calibration.yml --ref main -f source_sha="$(git rev-parse HEAD)"
```

Run the same comparison locally from the repository root:

```sh
docker compose exec -T -e PLANORAMA_SPACING_REPORT_DIR=output/spacing-calibration dev cargo test --locked --lib budget_calibration -- --ignored --nocapture
```

The test runs the current routing algorithm with the old numerical limits (2 sweeps,
6 attempts, 64 px growth), the [current limits](routing.md), and doubled limits on
the same initial geometry. It covers the large Terraform fixture, dense parallel
corridors, and protected junctions. Compare attempts, accepted expansions, residual
spacing deficits, canvas dimensions and per-edge bends, length and backtracking in
`spacing-calibration.json`. Inspect the three SVG variants for each fixture.
CI retains the JSON and nine SVGs in `spacing-calibration-<SHA>` for 14 days.
Timings are observational. Recalibrate when workloads or routing policy change.

## When generation runs

Rendering, generation-input, fixture or SVG-generation changes require Docker
`sh scripts/update-examples.sh`, even when no SVG difference is expected.
Inspect the diff and affected diagrams, then commit necessary outputs with code.
The [manifest](../scripts/generated-svgs.txt) is the generation/writeback authority.
Adding an example may also require its input command in the script.

PR creation/push does not regenerate all SVGs. The local script also refreshes
examples/coverage.md; manual writeback covers only listed SVGs. HTML remains an
explicit generation step. The combined multi-account example uses release mode;
its expensive full snapshot is opt-in, with fast topology coverage on ordinary PRs.

### Ordinary snapshot coverage

Ordinary Rust tests compare selected snapshots and behavioral assertions, not every
SVG in the manifest. Passing Tests/Clippy/browser checks does not establish that
all generated examples are current. Use the generation script when required.

## Generation and review

For remote regeneration or recovery:

```sh
gh workflow run update_examples.yml --ref main -f pull_request=123
```

The workflow ref selects the definition; the open PR's captured head is the
source. Rerunning may capture a newer head: verify the summary. Manual workflows
must exist on the default branch. Generation uploads `svgs/examples/`,
`updates.patch` and revision-bound `updates.json` for 14 days.

Writable same-repository branches receive a single SVG-only bot commit and
explicit validation dispatch. No diff means no commit. Failures publish no partial
changes. For a fork or restricted branch, apply the artifact on its matching source:

```sh
git apply --check /path/to/updates.patch
git apply /path/to/updates.patch
git add -- examples
git diff --cached
git commit -m "chore: regenerate SVG examples"
git push
```

Preserve unrelated changes and inspect staging before committing.

## Branch updates and permissions

Generation is read-only. A separate trusted writeback runner treats artifacts as
data, reads the source allowlist and never executes PR code or artifact scripts.
It atomically updates the branch only if the captured head is still current;
intervening pushes require regeneration. Forks use the patch fallback.

## Validation on the generated head

SVG bot commits suppress duplicate PR runs and explicitly dispatch validation.
Each dispatch binds PR, branch and expected SHA; merge checkout also verifies the
current base/head. A stacked predecessor is accepted only when its current PR and
merge tree prove the advertised base. Conflicts, stale heads or unavailable merge
refs fail rather than validating another revision.

### PR results and Actions jobs

Dispatched Actions jobs do not directly provide PR checks. **Report PR validation**
publishes commit statuses for the exact current generated head. `Dispatched Tests`
is reported as `Tests` (likewise Formatting, Clippy, applicable MSRV and Workflow
tests). Both test suites and their aggregate must succeed. Missing/skipped jobs
cannot report success. Only the latest eligible run/attempt may update statuses.
**SVG validation dispatch** confirms dispatch requests, not test success.

Require stable check/status names only where they run for every covered PR.
Do not require the reporter, `Dispatched ...`, optional generation or conditional
checks globally. Commit statuses do not satisfy rules requiring a workflow run.

### Recovery

| Situation | Action |
| --- | --- |
| Generation fails or SVGs are stale | Fix and regenerate locally, or push then dispatch Update SVG examples with the PR number. A push alone does not regenerate. |
| SVG commit exists but dispatch failed | Dispatch Update SVG examples again. No-diff repair on the same head starts validation without another commit. |
| Validation fails | Inspect the linked run; fix/push code failures, retry transient or merge-ref timing failures. |
| Status stays pending or reporter fails | Inspect/retry Report PR validation. A reporting API failure does not require rebuilding. |
| A push supersedes the generated head | Follow the new head's validation; prior results do not transfer. |

A manual validation without both PR inputs is ordinary branch validation and
cannot repair generated-head reporting. If no reporter run appears, inspect its
trigger on the default branch: a PR-only edit cannot activate it.

### Reporter verification

Workflow tests cover checkout, dispatch and reporting with Node tests and pinned
actionlint. After merging reporter changes, verify success/failure/retry statuses
on an SVG bot commit, with links to matching-SHA runs. A PR-only reporter cannot
demonstrate the complete trusted default-branch cycle.

## Validation after merging

Tests/Clippy seed caches on relevant pushes to `main`, including merges and direct
pushes, using the pushed commit and read-only permissions. Routing regressions
also run on `main` pushes, nightly and manually. These workflows do not use
`pull_request_target` and need no execution policy exception.

[Skip directives](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs)
such as `[skip ci]` suppress push validation if retained in a squash-merge message.
Remove them before merging, or dispatch Tests/Clippy and Routing regressions on
`main` afterward with PR inputs empty. Formatting/MSRV retain PR/manual triggers.

## Browser Wasm (#432)

**Browser Wasm** checks native/Wasm builds, UI parity and standalone distribution.
Use [browser verification](browser-development.md#verification) and
[engine memory contracts](browser-engine.md#per-generation-memory-selection).
`WASM_CI_SUITE=pr sh scripts/wasm-ci.sh` runs representative SVG/HTML parity,
UI generation/downloads, selection, CSP/sandbox/offline checks, memory/trap recovery,
and cancellation during real large-input Wasm routing followed by retry and stale
notification checks. Settings/reset/file-change/error transitions use a controlled
Worker. Native `artwork_free::` and CLI help tests run in ordinary all-feature Rust
CI: default features are empty and `icon-artwork` is a no-op, with no conditional
implementations or test exclusions. Reassess this if feature behavior changes.

`WASM_CI_SUITE=full` (also the script default) additionally runs service-specific
SVG/HTML comparisons, large generation completion/native parity, all standalone
fixtures, and the complete no-default-feature library suite plus CLI help tests.
**Wasm engine release** runs this scope on every `release` branch update and manual
invocation. It does not publish on push. Manual **Browser Wasm** accepts `suite:
full` for the same validation. Invalid scope values fail rather than skip checks.

Both distributions share one explicit no-default-feature native reference and one
Wasm build, binding generation and memory adapter. Their copied engine files are
checked for byte equality before browser and extracted-package validation.

### Browser Wasm caches and measurements (#674)

Cache hits never bypass build or validation. Delete affected `wasm-ci-v3-*` entries
(or bump the prefix) for Cargo recovery; delete the `browser-wasm` BuildKit cache
for environment reset. Bindings/archives must use freshly built workspace code.
No external cache service or secrets are needed.

`sh scripts/wasm-ci.sh` writes phase/bindings/browser TSVs, selection and environment
records to output/wasm-ci (override with repository-relative WASM_CI_REPORT_DIR).
The seven-day artifact excludes input/generated content. Compare complete job
durations as well as phase timing: setup/cache transfers add costs. Isolate target
and compiler-cache directories for controlled cold/warm comparisons; do not compare
cold full tests against warm focused tests as if scope were their only difference.

## Native Rust test coverage

Install the pinned tool with `cargo install --locked cargo-llvm-cov --version 0.6.21`
and add `llvm-tools-preview` to the repository Rust toolchain. Run
`sh scripts/rust-coverage.sh` in the development environment, or pass `html`
for the source browser. The manual **Rust coverage** workflow retains the LLVM
JSON and optional HTML for seven days. Read the file/region records under
`output/rust-coverage/` when deciding whether an untested behavior needs a case.

Measurement uses the normal non-ignored library, binary, integration and example
targets with all features, including `large_example`. The report excludes test
and example source from the denominator. Browser JS/Wasm execution, deferred
routing benchmarks and opt-in saved-plan/tool runs have their own checks.
Stable Rust supplies line and region counts here. Branch instrumentation requires
an unstable toolchain. No percentage gates the PR suite.
Line and region counts come from stable Rust's LLVM source coverage. Branch
coverage requires a different toolchain. A covered line does not establish
that both outcomes of a proof or scope check were exercised.

## Reference and provider properties

The native library suite runs three reference and proof properties with 24 cases
each, seed 1372, and small keys and resource sets. Budget one second of test
execution on a warm build. To replay a failure, use the printed minimized input
and run the same test with its seed, for example
`PROPTEST_RNG_SEED=1372 cargo test --locked --all-features --lib reference_properties`.
Use `PROPTEST_CASES=256` with an explicit seed for a larger local run. Keep a
minimized case as a regression when it exposes a distinct product bug.

## Targeted Rust mutation tests

Install `cargo-mutants` 25.3.1 with `cargo install --locked cargo-mutants --version 25.3.1`.
Run a pilot manually from the repository root:

| Command | Selection and observations | Output directory |
| --- | --- | --- |
| `sh scripts/rust-mutants.sh reference` (default) | Three instance-key, Cloudflare provider-binding and AWS certificate predicates. Runs the selected library tests. | `output/rust-mutants` |
| `sh scripts/rust-mutants.sh diff` | Four mutations in resource matching. Invert unique predecessor count, provider equality and mode equality individually, then change the ambiguous-endpoint filter from AND to OR. Runs `cargo test --locked --all-features --test cli -- diff_`; the integration tests invoke the rebuilt Planorama binary. | `output/rust-mutants/diff` |
| `sh scripts/rust-mutants.sh input-file` | Invert file-handle equality. The CLI test checks that single and named inputs keep their original bytes when output aliases the input, and that a separate file accepts output. | `output/rust-mutants/input-file` |
| `sh scripts/rust-mutants.sh sensitive-path` | Require both sensitivity masks on an object key and check the serialized change and replacement paths. | `output/rust-mutants/sensitive-path` |
| `sh scripts/rust-mutants.sh anonymous-metadata` | Skip anonymization of change metadata. The CLI tests inspect all exported bytes in SVG, HTML and JSON and exercise filtered HTML. | `output/rust-mutants/anonymous-metadata` |
| `sh scripts/rust-mutants.sh remote-output` | Weaken the completeness check for remote-state output provenance. Compare cross-state edges and unresolved-output diagnostics for explicit mappings. | `output/rust-mutants/remote-output` |
| `sh scripts/rust-mutants.sh service-scope` | Require both a global role and a matching region. Check service links for matching and mismatched provider scopes. | `output/rust-mutants/service-scope` |
| `sh scripts/rust-mutants.sh saved-provider` | Weaken the exclusion of deposed objects from saved-plan provider evidence. Compare the verified accounts of current and historical objects. | `output/rust-mutants/saved-provider` |
| `sh scripts/rust-mutants.sh drift-identity` | Match drift by address or deposed key. Check current and historical drift, planned actions, input order and unmatched diagnostics. | `output/rust-mutants/drift-identity` |
| `sh scripts/rust-mutants.sh cached-data` | Invert the data-resource check for prior-state fallback. Compare cached ID evidence with explicit nulls and masked ARN values. | `output/rust-mutants/cached-data` |
| `sh scripts/rust-mutants.sh memory-headroom` | Invert the memory limit comparison. Check exact fit, one-byte excess and checked addition overflow using host-side arithmetic. | `output/rust-mutants/memory-headroom` |
| `sh scripts/rust-mutants.sh json-budget` | Require both structural limits to be exceeded. Check nesting depth, element count, malformed JSON and the shared budget for named inputs. | `output/rust-mutants/json-budget` |
| `sh scripts/rust-mutants.sh named-limits` | Reject exactly 64 named inputs. Check input count, combined byte budget and duplicate state names. | `output/rust-mutants/named-limits` |
| `sh scripts/rust-mutants.sh pseudonym-collision` | Weaken the reserved-name check. Inspect anonymous identifiers and verify that relationship endpoints resolve in the projected graph. | `output/rust-mutants/pseudonym-collision` |
| `sh scripts/rust-mutants.sh module-instance` | Weaken module name and instance-key matching. Compare unkeyed selections, specific instances and their descendants. | `output/rust-mutants/module-instance` |
| `sh scripts/rust-mutants.sh focus-depth` | Remove the unseen-neighbor check in focus traversal. Compare CLI selections at successive depths and the ancestors retained for context. | `output/rust-mutants/focus-depth` |
| `sh scripts/rust-mutants.sh changed-context` | Weaken the check for a selected child with placement evidence. Inspect lowered relationships, cross-state outputs and retained parents under changed-only selection. | `output/rust-mutants/changed-context` |
| `sh scripts/rust-mutants.sh coverage-scope` | Weaken the managed AWS scope check. Compare coverage denominators and selected resource counts after filtering. | `output/rust-mutants/coverage-scope` |
| `sh scripts/rust-mutants.sh check-target` | Weaken the mode and type checks on Terraform check targets. Compare instance associations and preserved check statuses. | `output/rust-mutants/check-target` |
| `sh scripts/rust-mutants.sh plan-action` | Allow either create or delete to trigger replacement. Compare planned action classifications for individual operations and both replacement orders. | `output/rust-mutants/plan-action` |
| `sh scripts/rust-mutants.sh containment-parent` | Invert the containment edge check. Compare parents, roots and children for dependency-only nodes, nested containers, competing parents and cycles in both edge orders. | `output/rust-mutants/containment-parent` |
| `sh scripts/rust-mutants.sh generation-slice` | Reject the largest valid generation slice. Compare named SVG, HTML and diagnostics with one-shot output and exercise cancellation and failed generation. | `output/rust-mutants/generation-slice` |
| `sh scripts/rust-mutants.sh routing-search-budget` | Add each routing search charge to the remaining budget. Check interleaved jobs with budgets of three and four units and the cumulative cost of two named states. | `output/rust-mutants/routing-search-budget` |
| `sh scripts/rust-mutants.sh state-svg-ids` | Drop state prefixes from nested SVG IDs. Check unique IDs and state-local resolution of fragment links and arrow markers. | `output/rust-mutants/state-svg-ids` |
| `sh scripts/rust-mutants.sh frame-contact` | Suppress positive straight overlap along frame borders. Compare free crossings and normal endpoint stubs with parallel contacts. | `output/rust-mutants/frame-contact` |
| `sh scripts/rust-mutants.sh port-collision` | Invert the distinct-edge check for coincident source and target ports. Check node scoping and the final ports of reciprocal connections. | `output/rust-mutants/port-collision` |

Each run requires the expected mutant count, executes an unmodified baseline in
a temporary copy and checks that the baseline ran at least one passing test.
The mutation run has a 20-minute limit, seven minutes per build and 90 seconds
per test phase. The pilots are independent of normal PR jobs.

Inspect `mutants.out/outcomes.json` and the logs under the selected output directory.
For a survivor, confirm the selected tests exercise the affected path and that
the replacement changes observable behavior. Classify equivalent mutations,
build failures and timeouts separately. Expand the pilot only if a new,
non-equivalent survivor identifies a distinct assertion gap within the budget.
Stop expanding when the results duplicate existing regressions or only produce
equivalent, unbuildable or timed-out mutants.

## Test inventory

The tab-separated files under `tests/inventory/` are the maintained catalog.
Each row describes an area, behavior, input or operation, expected observation,
repository-relative source, case identifier and runner selector. Edit these
rows when an assertion or fixture changes the contract. Keep input conditions
and observable results specific enough to compare assertions across layers.

From the repository root, in the development environment:

```sh
node scripts/test-inventory.cjs list
node scripts/test-inventory.cjs check
node scripts/test-inventory.cjs check --cargo
node scripts/test-inventory.cjs html
```

`list` prints registered keys for adding or renaming rows. The fast check reads
Rust declarations, collects Node test registrations without invoking their test
callbacks, enumerates viewer methods, and reads Wasm and fixture case tables.
`check --cargo` also compiles the locked, all-feature Cargo test targets and
compares each executable's libtest discovery output (including source, qualified
name and ignore status). No test body runs in this step. The pinned toolchain's
JSON discovery switch is enabled only when listing the compiled executable.
Use `list --cargo` for exact Rust selectors. In the TSV the first selector segment
is the Cargo target; the remainder is the libtest name.

Open `output/test-inventory/index.html` locally. The page embeds the TSV data,
can filter by type, area or terms across behavior, input, observation and case,
and links to local sources. Rebuild it after editing the TSVs. Scope and counting:

- Unit, integration, property and ignored benchmark registrations have one row per
  Rust test. An ignored row may still run in a dedicated job. Tables inside one
  function share the runner selector; follow the function or named fixture to its
  individual inputs. Seeded property trials share the parameterized property.
- Node's named registrations include expanded case names. Browser entries cover
  viewer methods and browser checks. Wasm matrices include fixture, scenario
  and format where independently selected, including SVG-only cases. A script
  may also have assertions outside its matrix. Entries include directly runnable
  suites as well as suites selected by the PR or full CI workflow.
- Compatibility, layout, emulator, CI and tooling checks have their own tables.
  Lean proofs, drivers, input generators and support libraries do not count as
  executable test registrations. Rust consumers of shared Lean case data are
  included. The Lean commands are in [CONTRIBUTING.md](../CONTRIBUTING.md).

Mutation outcomes are suite-level measurements in `mutation-results.tsv`. A
missing measurement for a behavior and revision means **not evaluated**. A
survivor requires a mutant that built and passed the selected tests. The six
historical rows come from the pre/post-property runs in [PR #1378](https://github.com/ritz-p/planorama/pull/1378);
the report identifies the two test selections but does not record the tested
commit SHAs. Their revision is explicitly marked `unrecorded`.

For a new cargo-mutants 25.3.1 run, record the clean tested SHA and the actual
test selection at execution time. Convert its machine-readable report to TSV
rows with the following command. Review the behavior labels, then append the
rows to `tests/inventory/mutation-results.tsv`. Keep the report and logs in the
run artifact. A baseline failure provides no mutant detection evidence.

```sh
node scripts/test-inventory/mutation-report.cjs \
  output/rust-mutants/mutants.out/outcomes.json "$tested_sha" "$test_selection" "$artifact_reference"
```
