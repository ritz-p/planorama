const test = require('node:test');
const assert = require('node:assert/strict');
const {requireLayoutQuality, uploadAssets} = require('../../.github/scripts/wasm-release.cjs');

function uploadFixture(failure) {
  const assets = ['engine.tar.gz', 'engine.tar.gz.sha256'].map(name => {
    const data = Buffer.from(name);
    return {name, data, digest: require('node:crypto').createHash('sha256').update(data).digest('hex')};
  });
  const existing = [];
  const uploads = [];
  let failed = false;
  const github = {
    paginate: async () => existing,
    rest: {repos: {
      listReleaseAssets: () => {},
      uploadReleaseAsset: async ({name, data}) => {
        const shouldFail = name.endsWith('.sha256') && !failed;
        if (shouldFail) failed = true;
        if (shouldFail && failure === 'before') throw new Error('transport failed');
        const remote = {name, size: data.length, digest: `sha256:${require('node:crypto').createHash('sha256').update(data).digest('hex')}`};
        existing.push(remote);
        uploads.push(name);
        if (shouldFail && failure === 'after') throw new Error('response lost');
        return {data: shouldFail && failure === 'verification' ? {...remote, digest: null} : remote};
      },
    }},
  };
  return {assets, existing, uploads, github};
}

for (const failure of ['before', 'after', 'verification']) {
  test(`engine upload resumes after ${failure} failure without replacing assets`, async () => {
    const {github, assets, existing, uploads} = uploadFixture(failure);
    await assert.rejects(uploadAssets(github, {}, 1, assets));
    assert.equal(existing[0].name, assets[0].name);
    await uploadAssets(github, {}, 1, assets);
    await uploadAssets(github, {}, 1, assets);
    assert.deepEqual(uploads, assets.map(asset => asset.name));
    assert.equal(existing.length, 2);
  });
}

test('conflicting existing asset blocks all uploads before any mutation', async () => {
  const {github, assets, existing, uploads} = uploadFixture();
  existing.push({name: assets[1].name, size: assets[1].data.length, digest: 'sha256:wrong'});
  await assert.rejects(uploadAssets(github, {}, 1, assets), /Existing asset differs/);
  assert.deepEqual(uploads, []);
});

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
  let run = {id: 1, event: 'workflow_dispatch', head_sha: 'expected', conclusion: 'success'};
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
  run = {...run, event: 'pull_request'};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'));
  run = {...run, event: 'workflow_dispatch'};
  job = {conclusion: 'skipped', steps: []};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'), /comparison is required/);
  job = {conclusion: 'success', steps: [{name: 'Compare final layouts', conclusion: 'skipped'}]};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'));
  run = {...run, head_sha: 'stale'};
  await assert.rejects(requireLayoutQuality(github, {}, 'expected'));
});
