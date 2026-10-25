const {test} = require('node:test');
const assert = require('node:assert/strict');
const {mkdtempSync, writeFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const {join, resolve} = require('node:path');
const {spawnSync} = require('node:child_process');
const {shouldSave} = require('../../.github/scripts/compiler-cache.cjs');

test('cache uploads require newly written entries and complete known statistics', () => {
  const record = {version: '0.10.0', stats: {cache_writes: 3, cache_write_errors: 0}};
  assert.equal(shouldSave(record), true);
  assert.equal(shouldSave({...record, stats: {cache_writes: 0, cache_write_errors: 0}}), false);
  assert.equal(shouldSave({...record, stats: {cache_writes: 3, cache_write_errors: 1}}), false);
  assert.throws(() => shouldSave({...record, version: 'other'}));
  assert.throws(() => shouldSave({...record, stats: {}}));
});

test('compiler wrapper forwards arguments and errors, and works without sccache', () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-rustc-cache-'));
  try {
    const compiler = join(directory, 'rustc');
    writeFileSync(compiler, '#!/bin/sh\nprintf "%s\\n" "$@"\nexit 23\n', {mode: 0o755});
    const run = (incremental = '') => spawnSync('/bin/sh', [resolve('scripts/rustc-cache.sh'), compiler, 'file with spaces.rs'], {
      encoding: 'utf8', env: {...process.env, PATH: directory, CARGO_INCREMENTAL: incremental},
    });
    assert.equal(run().status, 23);
    assert.equal(run().stdout, 'file with spaces.rs\n');
    writeFileSync(join(directory, 'sccache'), '#!/bin/sh\nprintf "cached\\n"\nexec "$@"\n', {mode: 0o755});
    assert.equal(run().status, 23);
    assert.equal(run().stdout, 'cached\nfile with spaces.rs\n');
    assert.equal(run('1').stdout, 'file with spaces.rs\n');
    assert.equal(run('1').status, 23);
  } finally {
    rmSync(directory, {recursive: true, force: true});
  }
});
