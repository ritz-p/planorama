const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {createHash} = require('node:crypto');
const {targets, archiveName, validateVersion, collect, publish} = require('../../.github/scripts/release.cjs');
const hash = data => createHash('sha256').update(data).digest('hex');

function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-release-'));
  t.after(() => fs.rmSync(directory, {recursive: true, force: true}));
  for (const target of targets) {
    const name = archiveName('0.1.0', target);
    fs.writeFileSync(path.join(directory, name), target);
    fs.writeFileSync(path.join(directory, `${name}.sha256`), `${hash(target)}  ${name}\n`);
  }
  return directory;
}

function api({existing = [], failUpload = false, badDigest = false} = {}) {
  const calls = [];
  const assets = [];
  const repos = {
    listReleases: () => existing,
    listReleaseAssets: () => assets,
    createRelease: async args => { calls.push(['create', args]); return {data: {id: 42}}; },
    uploadReleaseAsset: async args => {
      calls.push(['upload', args.name]);
      if (failUpload) throw new Error('upload failed');
      const asset = {name: args.name, size: args.data.length, digest: `sha256:${badDigest ? 'bad' : hash(args.data)}`};
      assets.push(asset);
      return {data: asset};
    },
    updateRelease: async args => { calls.push(['publish', args]); },
  };
  return {calls, rest: {repos}, paginate: async fn => fn()};
}
const context = {repo: {owner: 'owner', repo: 'repo'}, eventName: 'push', ref: 'refs/tags/v0.1.0', sha: 'abc'};
const options = (directory, github) => ({directory, github, context, version: '0.1.0', tag: 'v0.1.0'});

test('only an exact stable package tag is accepted', () => {
  validateVersion('0.1.0', 'v0.1.0');
  for (const tag of ['v0.2.0', '0.1.0', 'v0.1.0-rc.1', 'v0.1.0\n']) assert.throws(() => validateVersion('0.1.0', tag));
});

test('all target checksums are required; changed bytes and extra assets fail', t => {
  const directory = fixture(t);
  assert.equal(collect(directory, '0.1.0').length, 5);
  const archive = path.join(directory, archiveName('0.1.0', targets[0]));
  fs.appendFileSync(archive, 'corruption');
  assert.throws(() => collect(directory, '0.1.0'), /Checksum/);
  fs.unlinkSync(archive);
  assert.throws(() => collect(directory, '0.1.0'), /Incomplete/);
});

test('PRs, mismatch, missing targets and existing draft/published releases never mutate releases', async t => {
  const directory = fixture(t);
  for (const overrides of [{context: {...context, eventName: 'pull_request'}}, {tag: 'v0.2.0'}]) {
    const github = api();
    await assert.rejects(publish({...options(directory, github), ...overrides}));
    assert.deepEqual(github.calls, []);
  }
  for (const draft of [false, true]) {
    const github = api({existing: [{tag_name: 'v0.1.0', draft}]});
    await assert.rejects(publish(options(directory, github)), /already exists/);
    assert.deepEqual(github.calls, []);
  }
  fs.unlinkSync(path.join(directory, archiveName('0.1.0', targets[0])));
  const github = api();
  await assert.rejects(publish(options(directory, github)), /Incomplete/);
  assert.deepEqual(github.calls, []);
});

test('failed or corrupt uploads leave the release in draft', async t => {
  const directory = fixture(t);
  for (const failure of [{failUpload: true}, {badDigest: true}]) {
    const github = api(failure);
    await assert.rejects(publish(options(directory, github)));
    assert.equal(github.calls[0][1].draft, true);
    assert.equal(github.calls.some(([name]) => name === 'publish'), false);
  }
});

test('only the complete verified set is published after uploading', async t => {
  const github = api();
  await publish(options(fixture(t), github));
  assert.deepEqual(github.calls.map(([name]) => name), ['create', 'upload', 'upload', 'upload', 'upload', 'upload', 'publish']);
  assert.equal(github.calls.at(-1)[1].draft, false);
});
