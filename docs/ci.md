# SVG generation in pull requests

The **Update SVG examples** workflow regenerates the maintained diagrams when a PR
changes their inputs. Contributors can push source changes and let Actions append
the generated SVG changes to the same PR branch.

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
4. For a writable branch in this repository, append one
   `chore: regenerate SVG examples [skip ci]` commit containing only changed SVGs.
5. Explicitly start the applicable validation workflows for the resulting PR head.
   The workflow summary links each run; a dispatch confirmation is not a passing
   validation result.

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
performs writeback, with `contents: write`, `actions: write` and
`pull-requests: read`. The workflow uses inline JavaScript through
`actions/github-script` for preparation, packaging and writeback. The write-enabled
job reads the output allowlist again from the captured source commit through the
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

## Validation and recovery

The standard `GITHUB_TOKEN` is used for both writeback and explicit
`workflow_dispatch` requests. The bot commit's `[skip ci]` directive suppresses
duplicate push/PR-event runs, including the token-generated PR runs that would
otherwise require approval. Explicit dispatch is unaffected by that directive and
starts validation on the generated head without an additional approval in the
ordinary same-repository flow. The directive does not cancel runs already created
for an earlier commit.

Formatting, Clippy and Tests are dispatched after an SVG commit. MSRV is included
when the full PR file list touches its existing toolchain/Cargo/Docker inputs,
MSRV workflow or shared validation checkout helper. This validation selection
examines the whole PR, including changes before the generated commit, independently
of the narrower generation inputs above.

Dispatch specifies the PR branch, PR number and exact expected head SHA. The
returned run ID is checked against that SHA, event and workflow. When these PR
inputs are provided, each validation workflow also checks its own run/checkout
SHA, fetches the PR merge ref, verifies
its parents against the expected head and current base, and checks the PR metadata
again. This preserves PR/base merge validation while attaching the workflow run
to the generated head. Short, bounded retries allow GitHub's PR metadata and merge
ref to catch up after the bot commit. A stale dispatch, a conflicting PR or a merge
ref that still has not caught up fails explicitly instead of testing a different
revision.

If validation dispatch fails after a commit, run **Update SVG examples** manually
with the PR number. An unchanged head carrying the generated-commit marker, with
or without the newer `[skip ci]` suffix, restarts validation without making another
commit. If the code itself fails a check, fix it on the PR branch and push
normally. A merge-ref timing failure can be retried from its validation run once
GitHub updates the merge ref.

Manual workflow dispatch is available after the workflow exists on the default
branch. During this workflow's initial implementation PR, generation runs through
the `pull_request` event; the four existing validation workflows can be dispatched
using their updated definitions on the PR branch. Before accepting the automation,
confirm in a real PR that an SVG-only bot commit receives the expected checks on
its new head and that the checks complete. PR branches created before this workflow
was introduced must first incorporate the updated generation tools and validation
workflow definitions from the base branch.

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

## Alternative credentials

If repository rules require event-based checks that explicit dispatch cannot
satisfy, a repository administrator can configure a GitHub App installation token
with narrowly scoped repository Contents and Actions write access and Pull requests
read access for writeback/dispatch. This requires modifying the writeback job to
mint an installation token and supply it to `actions/github-script`; setting
secrets alone does not change the standard-token implementation. Its App ID/private
key must be configured in Actions; a developer's ChatGPT GitHub connection is not
an Actions credential. The same output allowlist, atomic head check and fork
restrictions should remain.

References: [workflow triggers and GITHUB_TOKEN](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow),
[skip directives](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs),
[merged-PR events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request_target-workflow-when-a-pull-request-merges),
[workflow dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event),
[atomic Git ref updates](https://docs.github.com/en/graphql/reference/git#updaterefs).
