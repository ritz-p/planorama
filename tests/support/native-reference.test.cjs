const {test} = require('node:test');
const assert = require('node:assert/strict');
const {mkdtempSync, writeFileSync, rmSync} = require('node:fs');
const {join} = require('node:path');
const {tmpdir} = require('node:os');
const nativeReference = require('./native-reference.cjs');

test('native references distinguish bytes, complete options and executable changes', () => {
  const dir = mkdtempSync(join(tmpdir(), 'native-reference-'));
  try {
    const binary = join(dir, 'native');
    writeFileSync(binary, '#!/bin/sh\ncat\nprintf "%s" "$*"\n', {mode: 0o755});
    const run = nativeReference(binary);
    assert.equal(run('one', ['--format', 'svg']), 'one- -o - --format svg');
    run(Buffer.from('one'), ['--format', 'svg']);
    assert.deepEqual(run.stats, {hits: 1, misses: 1});
    assert.equal(run('two', ['--format', 'svg']), 'two- -o - --format svg');
    assert.equal(run('one', ['--format', 'html']), 'one- -o - --format html');
    writeFileSync(binary, '#!/bin/sh\ncat\nprintf changed\n');
    assert.equal(run('one', ['--format', 'svg']), 'onechanged');
    assert.deepEqual(run.stats, {hits: 1, misses: 4});
  } finally { rmSync(dir, {recursive: true, force: true}); }
});

test('failed references are retried and oversized output is not retained', () => {
  const dir = mkdtempSync(join(tmpdir(), 'native-reference-'));
  try {
    const binary = join(dir, 'native');
    writeFileSync(binary, '#!/bin/sh\nexit 1\n', {mode: 0o755});
    const run = nativeReference(binary, {limit: 2});
    assert.throws(() => run(''));
    assert.throws(() => run(''));
    writeFileSync(binary, '#!/bin/sh\ncat\n');
    run('large'); run('large');
    assert.deepEqual(run.stats, {hits: 0, misses: 4});
  } finally { rmSync(dir, {recursive: true, force: true}); }
});
