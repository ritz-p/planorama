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

## Terraform compatibility (#525)

[Compatibility checks](terraform-compatibility.md) validate committed fixtures on
relevant PRs; fixed-tool capture is manual. The separate provider-evidence job
verifies saved plans against local STS with external networking disabled. Its
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

The `spacing-calibration-<SHA>` artifact retains JSON and nine SVGs for 14 days.
[Local calibration](../examples/terraform-large/spacing-calibration.md) compares
budgets without changing maintained examples. Timings are observations, not gates.

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

Tests/Clippy seed caches after merged PR events using the default-branch commit,
read-only permissions and no PR-head execution. After a direct push, dispatch
Tests/Clippy on main with PR inputs empty. Formatting/MSRV retain PR/manual triggers.

## Browser Wasm (#432)

**Browser Wasm** checks native/Wasm builds, UI parity and standalone distribution.
Use [browser verification](browser-development.md#verification) and
[engine memory contracts](browser-engine.md#per-generation-memory-selection).
PR no-default native tests select `artwork_free::` plus CLI help; complete
all-feature Rust tests remain in ordinary CI. Feature-specific implementation
outside that selection requires reassessment. Run the full library with manual
`full_library: true` or `WASM_CI_FULL_LIB=1 sh scripts/wasm-ci.sh`; engine releases
retain the complete suite. Service parity and large completion remain on PRs
because native tests do not cover Wasm/worker boundaries.

### Browser Wasm caches and measurements (#674)

Cache hits never bypass build or validation. Delete affected `wasm-ci-v2-*` entries
(or bump the prefix) for Cargo recovery; delete the `browser-wasm` BuildKit cache
for environment reset. Bindings/archives must use freshly built workspace code.
No external cache service or secrets are needed.

`sh scripts/wasm-ci.sh` writes phase/bindings/browser TSVs, selection and environment
records to output/wasm-ci (override with repository-relative WASM_CI_REPORT_DIR).
The seven-day artifact excludes input/generated content. Compare complete job
durations as well as phase timing: setup/cache transfers add costs. Isolate target
and compiler-cache directories for controlled cold/warm comparisons; do not compare
cold full tests against warm focused tests as if scope were their only difference.
