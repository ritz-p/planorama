# SVG generation in pull requests

The **Update SVG examples** workflow regenerates the maintained diagrams when a PR
changes the renderer, build inputs, committed fixtures, examples or generation/CI
tools. Contributors can push source changes and let Actions append the generated
SVG changes to the same PR branch.

## Generation and review

1. Capture the open PR's exact head SHA and check out that revision.
2. Run `sh scripts/update-examples.sh` using the pinned Rust toolchain and locked
   dependencies. [`scripts/generated-svgs.txt`](../scripts/generated-svgs.txt) is
   the shared generation list and writeback allowlist.
3. Upload an artifact containing the complete SVG output under `svgs/examples/`,
   an `updates.patch` file, and revision-bound `updates.json` metadata.
4. For a writable branch in this repository, append one
   `chore: regenerate SVG examples` commit containing only changed SVGs.
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
`pull-requests: read`. It loads the tools from the workflow's own revision, reads
the output allowlist again from the captured source commit through the GitHub API,
and handles the artifact as data. It does not build the PR source, run downloaded
scripts or apply an artifact patch in the write-enabled job.

GitHub creates a tree and commit containing the declared SVG changes. The branch
update uses GraphQL `updateRefs` with the captured `beforeOid` and `force: false`.
If the head changes before this atomic update, including a branch rewind, the
update is rejected. The old result is discarded and the newer PR update receives
its own generation run. Per-PR concurrency does not cancel a running writeback
between its commit and validation dispatch.

Fork PRs keep the patch/artifact flow and do not enter the write-enabled job.
Protected branches, read-only tokens and repository policy restrictions also fall
back to the artifact. The workflow does not use `pull_request_target` or require a
personal token for the ordinary same-repository case.

## Validation and recovery

The standard `GITHUB_TOKEN` is used for both writeback and explicit
`workflow_dispatch` requests. A token-generated push does not automatically start
ordinary push workflows, and its PR-triggered workflows can require approval.
Explicit dispatch avoids relying on those event-triggered runs.

Formatting, Clippy and Tests are dispatched after an SVG commit. MSRV is included
when the full PR file list touches its existing toolchain/Cargo/Docker inputs or
the shared validation checkout helper. The selection examines the whole PR,
including changes before the generated commit.

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
with the PR number. An unchanged head carrying the generated-commit marker
restarts validation without making another commit. If the code itself fails a
check, fix it on the PR branch and push normally. A merge-ref timing failure can
be retried from its validation run once GitHub updates the merge ref.

Manual workflow dispatch is available after the workflow exists on the default
branch. During this workflow's initial implementation PR, generation runs through
the `pull_request` event; the four existing validation workflows can be dispatched
using their updated definitions on the PR branch. Before accepting the automation,
confirm in a real PR that an SVG-only bot commit receives the expected checks on
its new head and that the checks complete. PR branches created before this workflow
was introduced must first incorporate the updated generation tools and validation
workflow definitions from the base branch.

If repository rules require event-based checks that explicit dispatch cannot
satisfy, a repository administrator can configure a GitHub App installation token
with narrowly scoped repository Contents and Actions write access and Pull requests
read access for writeback/dispatch. This requires modifying the writeback job to
mint an installation token and pass it as `GH_TOKEN`; setting secrets alone does
not change the standard-token implementation. Its App ID/private key must be
configured in Actions; a developer's ChatGPT GitHub connection is not an Actions
credential. The same output allowlist, atomic head check and fork restrictions
should remain.

References: [workflow triggers and GITHUB_TOKEN](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow),
[workflow dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event),
[atomic Git ref updates](https://docs.github.com/en/graphql/reference/git#updaterefs).
