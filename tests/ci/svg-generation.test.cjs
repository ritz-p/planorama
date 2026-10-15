const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const {test} = require('node:test');

const workflow = readFileSync(path.join(__dirname, '../../.github/workflows/update_examples.yml'), 'utf8');
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const sha = 'a'.repeat(40);
const generated = 'b'.repeat(40);
const pr = () => ({state: 'open', head: {sha, ref: 'feature/example', repo: {full_name: 'owner/repo'}},
  base: {repo: {full_name: 'owner/repo', node_id: 'repository-id'}}});
function summary() {
  return {addRaw() {return this;}, addHeading() {return this;}, async write() {}};
}
function scriptAfter(marker) {
  const start = workflow.indexOf('          script: |', workflow.indexOf(marker));
  assert.ok(start >= 0);
  return workflow.slice(start).split(/\r?\n/).slice(1)
    .filter((line, index, lines) => lines.slice(0, index + 1).every(item => !item.trim() || item.startsWith('            ')))
    .map(line => line.slice(12)).join('\n');
}

test('SVG generation is manual-only with isolated read-only generation and allowlisted writeback', () => {
  const trigger = workflow.slice(workflow.indexOf('\non:'), workflow.indexOf('\npermissions:'));
  assert.match(trigger, /workflow_dispatch:/);
  assert.doesNotMatch(trigger, /^  (pull_request|push|pull_request_target):/m);
  assert.doesNotMatch(workflow, /context\.payload|successfulCertificate|generatedCommit|sourceInput/);
  const generation = workflow.slice(workflow.indexOf('  generate:'), workflow.indexOf('  writeback:'));
  assert.doesNotMatch(generation, /: write/);
  assert.match(generation, /ref: \$\{\{ steps.prepare.outputs.head_sha \}\}/);
  assert.match(generation, /persist-credentials: false/);
  assert.match(generation, /Generation checkout no longer matches the captured source SHA/);
  const writeback = workflow.slice(workflow.indexOf('  writeback:'));
  assert.match(writeback, /same_repository == 'true'/);
  assert.doesNotMatch(writeback, /actions\/checkout|execFile|exec\.exec|\brun:/);
  assert.match(writeback, /digest-mismatch: error/);
});

async function prepare(value = '405', pull = pr(), eventName = 'workflow_dispatch') {
  const outputs = {};
  await new AsyncFunction('github', 'context', 'core', 'process', scriptAfter('- name: Capture current PR head'))(
    {rest: {pulls: {get: async args => {assert.equal(args.pull_number, Number(value)); return {data: pull};}}}},
    {eventName, repo: {owner: 'owner', repo: 'repo'}},
    {setOutput: (key, value) => {outputs[key] = value;}, summary: summary()},
    {env: {SVG_PULL_REQUEST: value}},
  );
  return outputs;
}
test('manual preparation captures the latest exact head without a PR event payload', async () => {
  assert.deepEqual(await prepare(), {active: true, head_sha: sha, pull_request: 405,
    head_repo: 'owner/repo', same_repository: true});
  const fork = pr();
  fork.head.repo.full_name = 'contributor/fork';
  assert.equal((await prepare('405', fork)).same_repository, false);
});
test('preparation rejects invalid numbers, closed PRs, absent repositories and invalid SHAs', async () => {
  for (const number of ['', '-1', '0', '1e2', '9007199254740992']) {
    await assert.rejects(prepare(number), /Invalid pull request number/);
  }
  await assert.rejects(prepare('405', {...pr(), state: 'closed'}), /open PR/);
  await assert.rejects(prepare('405', {...pr(), head: {repo: null}}), /open PR/);
  await assert.rejects(prepare('405', {...pr(), head: {...pr().head, sha: 'abc'}}), /complete source commit SHA/);
  await assert.rejects(prepare('405', pr(), 'pull_request'), /Manual dispatch/);
});

async function writeback(options = {}) {
  const calls = [];
  let pull = pr();
  let pullReads = 0;
  const validatedSha = options.repair ? sha : generated;
  if (options.stale) pull.head.sha = 'c'.repeat(40);
  const bundle = {schema: 1, source_sha: sha, pull_request: 405,
    files: options.noDiff ? [] : [{path: 'examples/diagram.svg', content: '<svg/>'}], ...options.bundle};
  const fs = {lstat: async name => ({isDirectory: () => name === 'svg-artifact',
    isFile: () => name !== 'svg-artifact', isSymbolicLink: () => false, size: 100}),
  readFile: async () => Buffer.from(JSON.stringify(bundle))};
  const github = {
    request: async (route, args) => {
      const resource = route.split('/repos/{owner}/{repo}/')[1];
      calls.push({route, resource, args});
      let data;
      if (resource === 'pulls/405') {
        if (options.advance && ++pullReads > 1) pull.head.sha = 'c'.repeat(40);
        data = structuredClone(pull);
      }
      else if (resource === 'actions/artifacts/7') data = {expired: false, workflow_run: {id: 9}};
      else if (resource === 'contents/scripts/generated-svgs.txt') {
        assert.equal(args.ref, sha);
        data = {type: 'file', encoding: 'base64', content: Buffer.from('examples/diagram.svg\n').toString('base64')};
      } else if (resource === `git/commits/${sha}`) data = {tree: {sha: 'old-tree'},
        message: options.repair ? `chore: regenerate SVG examples [skip ci]\n\nPlanorama-SVG-Source: ${generated}` : 'source change'};
      else if (resource === 'git/trees') data = {sha: options.sameTree ? 'old-tree' : 'new-tree'};
      else if (resource === 'git/commits') data = {sha: generated};
      else if (resource === 'pulls/405/files') data = [];
      else if (resource.startsWith('statuses/')) data = {};
      else if (/actions\/workflows\/.+\/dispatches/.test(resource)) data = {workflow_run_id: 10};
      else if (resource === 'actions/runs/10') {
        const dispatch = calls.findLast(call => call.resource.endsWith('/dispatches'));
        const name = dispatch.resource.split('/')[2];
        data = {id: 10, head_sha: validatedSha, event: 'workflow_dispatch', head_branch: pull.head.ref,
          path: `.github/workflows/${name}`, display_title: `Validate PR #405 at ${validatedSha}`};
      } else throw new Error(`Unexpected request: ${route}`);
      return {data};
    },
    graphql: async (query, args) => {
      calls.push({resource: 'updateRefs', args});
      assert.deepEqual(args.input.refUpdates, [{name: 'refs/heads/feature/example',
        beforeOid: sha, afterOid: generated, force: false}]);
      if (options.protected) throw new Error('Protected branch');
      pull.head.sha = generated;
      return {updateRefs: {clientMutationId: null}};
    },
  };
  const invoke = () => new AsyncFunction('github', 'context', 'core', 'process', 'require',
    scriptAfter('- name: Commit SVG-only changes and dispatch checks'))(
    github, {repo: {owner: 'owner', repo: 'repo'}, runId: 9},
    {summary: summary(), info() {}, warning() {}},
    {env: {SVG_SOURCE_SHA: sha, SVG_PULL_REQUEST: '405', SVG_ARTIFACT_ID: '7'}},
    name => name === 'node:fs/promises' ? fs : require(name),
  );
  return {calls, invoke};
}
test('same-repository writeback uses atomic source-bound update and validates the generated head', async () => {
  const env = await writeback();
  await env.invoke();
  assert.equal(env.calls.filter(call => call.resource === 'updateRefs').length, 1);
  const dispatches = env.calls.filter(call => call.resource.endsWith('/dispatches'));
  assert.equal(dispatches.length, 3);
  assert.ok(dispatches.every(call => call.args.inputs.expected_head_sha === generated));
});
for (const options of [{noDiff: true}, {sameTree: true}, {stale: true}]) {
  test(`does not commit or dispatch for ${JSON.stringify(options)}`, async () => {
    const env = await writeback(options);
    await env.invoke();
    assert.ok(env.calls.every(call => !['git/commits', 'updateRefs'].includes(call.resource)
      && !call.resource.endsWith('/dispatches') && !call.resource.startsWith('statuses/')));
  });
}
test('protected branch failure preserves artifact fallback without dispatch', async () => {
  const env = await writeback({protected: true});
  await assert.rejects(env.invoke(), /Automatic writeback was rejected.*patch remains in the artifact/);
  assert.ok(env.calls.every(call => !call.resource.endsWith('/dispatches')));
});
test('a head advancing during writeback is never updated', async () => {
  const env = await writeback({advance: true});
  await env.invoke();
  assert.ok(env.calls.every(call => call.resource !== 'updateRefs' && !call.resource.endsWith('/dispatches')));
});
test('no-diff generated head repairs dispatch without another commit or resetting validation statuses', async () => {
  const env = await writeback({noDiff: true, repair: true});
  await env.invoke();
  assert.equal(env.calls.filter(call => call.resource.endsWith('/dispatches')).length, 3);
  assert.ok(env.calls.every(call => call.resource !== 'git/commits' && call.resource !== 'updateRefs'));
  assert.ok(env.calls.filter(call => call.resource.startsWith('statuses/'))
    .every(call => call.args.context === 'SVG validation dispatch'));
});
for (const bundle of [{source_sha: generated}, {pull_request: 406},
  {files: [{path: 'src/main.rs', content: 'untrusted'}]},
  {files: [{path: 'examples/../src/main.rs', content: 'untrusted'}]}]) {
  test(`rejects untrusted artifact ${JSON.stringify(bundle)}`, async () => {
    const env = await writeback({bundle});
    await assert.rejects(env.invoke(), /Artifact/);
    assert.ok(env.calls.every(call => !call.resource.startsWith('git/')));
  });
}
