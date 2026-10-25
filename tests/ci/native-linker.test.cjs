const {test} = require('node:test');
const assert = require('node:assert/strict');
const {mkdtempSync, writeFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const {join, resolve} = require('node:path');
const {spawnSync} = require('node:child_process');

test('native linker preserves arguments, selects mold explicitly and propagates driver failures', () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-linker-'));
  try {
    writeFileSync(join(directory, 'cc'), '#!/bin/sh\nprintf "%s\\n" "$@"\nexit "${DRIVER_STATUS:-0}"\n', {mode: 0o755});
    const run = (selection, status = '0') => spawnSync('sh', [resolve('scripts/native-linker.sh'), 'object with spaces.o', '-o', 'output file'], {
      encoding: 'utf8', env: {...process.env, PATH: `${directory}:${process.env.PATH}`,
        PLANORAMA_LINKER: selection, PLANORAMA_LINK_TIMINGS: '', DRIVER_STATUS: status},
    });
    const mold = run('mold');
    assert.equal(mold.status, 0);
    assert.equal(mold.stdout, '-fuse-ld=mold\n-Wl,--no-fork\nobject with spaces.o\n-o\noutput file\n');
    assert.equal(run('system').stdout, 'object with spaces.o\n-o\noutput file\n');
    assert.equal(run('mold', '37').status, 37);
    assert.equal(run('unknown').status, 2);
  } finally {
    rmSync(directory, {recursive: true, force: true});
  }
});
