const test = require('node:test');
const assert = require('node:assert/strict');
const {requireLayoutQuality} = require('../../.github/scripts/wasm-release.cjs');

test('standalone software notices do not copy artwork notices', t => {
  const fs = require('node:fs');
  const directory = fs.mkdtempSync(require('node:path').join(require('node:os').tmpdir(), 'engine-notices-'));
  t.after(() => fs.rmSync(directory, {recursive: true, force: true}));
  require('../../.github/scripts/release.cjs').notices([], directory, false);
  assert.deepEqual(fs.readdirSync(directory), ['THIRD-PARTY-NOTICES.txt']);
});

test('engine publication requires an executed comparison on the exact commit', async () => {
  const listWorkflowRuns = Symbol('runs');
  const listJobsForWorkflowRun = Symbol('jobs');
  let run = {id: 1, head_sha: 'expected', conclusion: 'success'};
  let job = {conclusion: 'success', steps: [{name: 'Compare final layouts', conclusion: 'success'}]};
  const github = {
    rest: {actions: {listWorkflowRuns, listJobsForWorkflowRun}},
    paginate: async (method, args) => {
      if (method === listWorkflowRuns) {
        assert.equal(args.workflow_id, 'layout_quality.yml');
        assert.equal(args.head_sha, 'expected');
        return [run];
      }
      assert.equal(args.run_id, 1);
      return [job];
    },
  };
  await requireLayoutQuality(github, {}, 'expected');
  job = {conclusion: 'skipped', steps: []};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'), /comparison is required/);
  job = {conclusion: 'success', steps: [{name: 'Compare final layouts', conclusion: 'skipped'}]};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'));
  run = {...run, head_sha: 'stale'};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'));
});
