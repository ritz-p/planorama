# SVG generation and PR validation

The **Update SVG examples** workflow regenerates the maintained diagrams when a PR
changes their inputs. Contributors can push source changes and let Actions append
the generated SVG changes to the same PR branch. Validation then runs on the new
head, and **Report PR validation** publishes its results to the PR.

## When generation runs

Automatic generation watches these inputs:

- Renderer and build inputs: `src/**`, `build.rs`, `assets/**`, `Cargo.toml`,
  `Cargo.lock`, `rust-toolchain.toml` and `.cargo/**`.
- Committed JSON inputs: `tests/fixtures/**/*.json` and `examples/**/*.json`.
- Snapshot fixture builders: `tests/unit/layout.rs`,
  `tests/unit/layout/containers.rs` and `tests/unit/layout/ports.rs`.
- Generation configuration: `scripts/generated-svgs.txt`,
  `scripts/update-examples.sh` and `.github/workflows/update_examples.yml`.
- Maintained SVGs listed in the manifest, so manual edits to generated output are
  checked against the current renderer.

For an ordinary PR update, the workflow inspects the latest push's changes before
building. A documentation-only or unrelated-test update can skip generation when
the preceding revision is known to have current SVGs, either from an SVG-only bot
commit or a successful workflow that confirmed its output was current. If that
cannot be confirmed, including when earlier generation is pending, failed or
superseded, the workflow checks the full PR diff and regenerates as needed. This
keeps an additional documentation push from discarding a pending renderer update.
Opening or reopening a PR uses its complete diff; a non-linear or incomplete push
comparison also falls back to that diff. An incomplete PR file list causes
generation conservatively. A manual dispatch with the PR number regenerates its
latest head regardless of the changed files.

When introducing another snapshot fixture builder or a non-JSON generation input,
add its path to both the workflow's trigger paths and its input matcher.

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
[same Docker/PowerShell entry points](../README.md#svg-の更新).

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
update is rejected. The old result is discarded and the newer PR update receives
its own generation run. Per-PR concurrency does not cancel a running writeback
between its commit and validation dispatch.

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
for an earlier commit.

Formatting, Clippy and Tests are dispatched after an SVG commit. MSRV is included
when the full PR file list touches its existing toolchain/Cargo/Docker inputs,
MSRV workflow or shared validation checkout helper. **Workflow tests** is included
for changes under `.github/**` or `tests/ci/**`. This validation selection
examines the whole PR, including changes before the generated commit, independently
of the narrower generation inputs above.

Dispatch specifies the PR branch, PR number and exact expected head SHA. Its run
name is `Validate PR #<number> at <full head SHA>`, and the returned run ID is checked
against that SHA, event and workflow. When these PR inputs are provided, each
validation workflow also checks its own run/checkout SHA, fetches the PR merge
ref, verifies its parents against the expected head and current base, and checks
the PR metadata again. This tests the PR/base merge while binding the run to the
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
allowed workflow path and ID, the PR number and expected SHA, and the dispatched
job's merge-checkout and validation steps. Only the latest eligible run/attempt
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

See GitHub's [required-check rules and event restrictions](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)
and [Commit Status API](https://docs.github.com/en/rest/commits/statuses).

### Recovery

| Situation | Action |
| --- | --- |
| SVG generation fails | Fix the reported problem and push, or retry **Update SVG examples** for a transient failure. |
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

### Rollout and verification

Manual dispatch requires the validation workflows to exist on the default branch.
Dispatch of the new Workflow tests workflow and the `workflow_run` reporter
become active only after their definitions are merged there. Workflow tests runs
on `pull_request` during its implementation PR. Those checks cannot demonstrate
the complete writeback/reporting cycle before that merge. PR branches created before the
change must incorporate the updated generation tools and validation definitions.

The separate [Workflow tests](../.github/workflows/workflow_tests.yml) workflow
runs the `Lint workflows` step with pinned actionlint and the
`Test validation reporting` step with Node's built-in test runner for
`tests/ci/**`. Rust builds, tests and SVG generation remain in their existing
CI workflows.
After the reporter is on the default branch, verify on a real PR that the
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
