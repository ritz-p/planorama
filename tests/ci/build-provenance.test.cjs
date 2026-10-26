const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const {execFileSync} = require('node:child_process');

test('build provenance handles clean, dirty, missing and parent Git metadata', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-provenance-'));
  try {
    const binary = path.join(temp, 'build-script');
    execFileSync('rustc', ['--edition=2024', 'build.rs', '-o', binary]);
    const root = path.join(temp, 'source');
    fs.mkdirSync(root);
    const git = (...args) => execFileSync('git', ['-c', `safe.directory=${root}`, '-C', root, ...args], {encoding: 'utf8'}).trim();
    const revision = (directory = root, extra = {}) => execFileSync(binary, [], {env: {...process.env, CARGO_MANIFEST_DIR: directory, ...extra}, encoding: 'utf8'}).match(/PLANORAMA_REVISION=(.*)/)[1];
    assert.equal(revision(), '');
    git('init', '-q');
    git('config', 'user.name', 'Fixture');
    git('config', 'user.email', 'fixture@example.invalid');
    assert.equal(revision(), '');
    fs.writeFileSync(path.join(root, 'source.rs'), 'fn main() {}');
    git('add', '.');
    git('commit', '-qm', 'fixture');
    const sha = git('rev-parse', 'HEAD');
    assert.equal(revision(), sha);
    assert.equal(revision(), sha);
    const ownerMismatch = {GIT_TEST_ASSUME_DIFFERENT_OWNER: '1'};
    assert.throws(() => execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], {env:{...process.env,...ownerMismatch},stdio:'pipe'}), error => /dubious ownership/.test(error.stderr.toString()));
    assert.equal(revision(root, ownerMismatch), sha);
    fs.appendFileSync(path.join(root, 'source.rs'), '\n');
    assert.equal(revision(), `${sha}-dirty`);
    assert.equal(revision(root, ownerMismatch), `${sha}-dirty`);
    git('add', '.');
    assert.equal(revision(), `${sha}-dirty`);
    git('commit', '-qm', 'changed');
    assert.equal(revision(), git('rev-parse', 'HEAD'));
    fs.writeFileSync(path.join(root, 'extra'), 'untracked');
    assert.equal(revision(), `${git('rev-parse', 'HEAD')}-dirty`);
    const archive = path.join(root, 'archive');
    fs.mkdirSync(archive);
    assert.equal(revision(archive), '');
    assert.equal(revision(root, {PATH: ''}), '');
  } finally {
    fs.rmSync(temp, {recursive: true, force: true});
  }
});
