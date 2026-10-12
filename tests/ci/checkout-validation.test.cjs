const assert = require('node:assert/strict');
const {mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync} = require('node:fs');
const {tmpdir} = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {test} = require('node:test');

const script = path.resolve(__dirname, '../../.github/scripts/checkout-validation.sh');
const repository = {full_name: 'ritz-p/planorama'};
const gitEnvironment = {
  ...process.env,
  GIT_AUTHOR_NAME: 'CI fixture', GIT_AUTHOR_EMAIL: 'fixture@example.test',
  GIT_COMMITTER_NAME: 'CI fixture', GIT_COMMITTER_EMAIL: 'fixture@example.test',
  GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1', GIT_TERMINAL_PROMPT: '0',
};

function git(cwd, args, input) {
  const result = spawnSync('git', args, {cwd, input, encoding: 'utf8', env: gitEnvironment});
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}

function setup(t, variant) {
  const directory = mkdtempSync(path.join(tmpdir(), 'planorama-checkout-'));
  t.after(() => rmSync(directory, {recursive: true, force: true}));
  const origin = path.join(directory, 'origin');
  const client = path.join(directory, 'client');
  const commands = path.join(directory, 'commands');
  for (const entry of [origin, client, commands]) mkdirSync(entry);
  git(origin, ['init', '--bare']);
  const tree = text => {
    const blob = git(origin, ['hash-object', '-w', '--stdin'], text);
    return git(origin, ['mktree'], `100644 blob ${blob}\tfixture.txt\n`);
  };
  const commit = (content, parents, message) => git(origin,
    ['commit-tree', content, ...parents.flatMap(parent => ['-p', parent])], `${message}\n`);
  const ancestor = commit(tree('ancestor\n'), [], 'ancestor');
  const baseTree = tree('base\n');
  const base = commit(baseTree, [ancestor], 'base');
  const headTree = tree('head\n');
  const head = commit(headTree, [base], 'head');
  const other = commit(tree('other\n'), [base], 'other');
  const predecessorMerge = commit(variant === 'changed_tree' ? tree('changed base\n') : baseTree,
    [ancestor, variant === 'wrong_ancestry' ? other : base], 'predecessor merge');
  const merge = commit(headTree,
    [variant === 'direct' ? base : predecessorMerge, variant === 'wrong_head' ? other : head], 'test merge');
  git(origin, ['update-ref', 'refs/pull/333/merge', merge]);
  git(origin, ['update-ref', 'refs/heads/feature/base', base]);
  git(origin, ['update-ref', 'refs/heads/feature/topic', head]);
  git(client, ['init']);
  git(client, ['remote', 'add', 'origin', `file://${origin}`]);
  git(client, ['fetch', '--depth=1', 'origin', head]);
  git(client, ['checkout', '--detach', head]);
  const pull = {
    number: 333, state: 'open', mergeable: true, merge_commit_sha: merge,
    head: {sha: head, ref: 'feature/topic', repo: repository},
    base: {sha: base, ref: 'feature/base', repo: repository},
  };
  const predecessor = {
    number: 329, state: 'open', mergeable: true, merge_commit_sha: predecessorMerge,
    head: {sha: base, ref: 'feature/base', repo: repository},
    base: {sha: ancestor, ref: 'main', repo: repository},
  };
  if (variant === 'different_branch') predecessor.head.ref = 'feature/unrelated';
  if (variant === 'fork') predecessor.head.repo = {full_name: 'someone/planorama'};
  const fresh = structuredClone(predecessor);
  if (variant === 'advanced_predecessor') fresh.base.sha = other;
  writeFileSync(path.join(directory, 'metadata.json'), JSON.stringify({
    pull, predecessors: variant === 'unregistered' ? [] : variant === 'ambiguous'
      ? [predecessor, {...predecessor, number: 328}] : [predecessor],
    fresh, other, advance: variant === 'advanced_current',
  }));
  writeFileSync(path.join(commands, 'gh'), `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const directory = process.env.PLANORAMA_CHECKOUT_FIXTURE;
const data = JSON.parse(fs.readFileSync(path.join(directory, 'metadata.json'), 'utf8'));
const endpoint = process.argv.find(value => value.startsWith('repos/'));
fs.appendFileSync(path.join(directory, 'requests'), endpoint + '\\n');
if (endpoint.endsWith('/pulls/333')) {
  const counter = path.join(directory, 'current-reads');
  const reads = fs.existsSync(counter) ? Number(fs.readFileSync(counter, 'utf8')) : 0;
  fs.writeFileSync(counter, String(reads + 1));
  if (data.advance && reads > 0) data.pull.head.sha = data.other;
  process.stdout.write(JSON.stringify(data.pull));
} else if (endpoint.endsWith('/pulls/329')) {
  process.stdout.write(JSON.stringify(data.fresh));
} else if (endpoint.endsWith('/pulls')) {
  process.stdout.write(JSON.stringify(data.predecessors));
} else {
  throw new Error('Unexpected endpoint: ' + endpoint);
}
`, {mode: 0o755});
  writeFileSync(path.join(commands, 'sleep'), '#!/usr/bin/env bash\nexit 0\n', {mode: 0o755});
  return {
    head, merge, client, directory,
    invoke: () => spawnSync('bash', [script], {
      cwd: client, encoding: 'utf8', timeout: 30000,
      env: {
        ...gitEnvironment, PATH: `${commands}:${process.env.PATH}`,
        PLANORAMA_CHECKOUT_FIXTURE: directory,
        GITHUB_REPOSITORY: repository.full_name, GITHUB_SERVER_URL: 'https://github.com',
        GITHUB_SHA: head, GH_TOKEN: 'fixture-token', PULL_REQUEST: '333', EXPECTED_HEAD_SHA: head,
      },
    }),
  };
}

for (const variant of ['direct', 'predecessor']) {
  test(`checks out the verified ${variant} PR merge from a shallow initial checkout`, t => {
    const environment = setup(t, variant);
    const result = environment.invoke();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(git(environment.client, ['rev-parse', 'HEAD']), environment.merge);
    const requests = readFileSync(path.join(environment.directory, 'requests'), 'utf8');
    assert.equal(requests.includes('/pulls/329'), variant === 'predecessor');
  });
}

for (const variant of ['changed_tree', 'wrong_ancestry', 'wrong_head', 'unregistered',
  'ambiguous', 'different_branch', 'fork', 'advanced_predecessor', 'advanced_current']) {
  test(`rejects ${variant} without checking out an unverified merge`, t => {
    const environment = setup(t, variant);
    const result = environment.invoke();
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /different head SHA|merge ref has not caught up/);
    assert.equal(git(environment.client, ['rev-parse', 'HEAD']), environment.head);
  });
}
