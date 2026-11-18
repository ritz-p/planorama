'use strict';

const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

function directory(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'emulator-guards-'));
  t.after(() => fs.rmSync(dir, {recursive: true, force: true}));
  return dir;
}

test('artifact check fails without leaking a matched credential', t => {
  const dir = directory(t);
  const out = path.join(dir, 'out');
  fs.mkdirSync(out);
  const key = path.join(dir, 'credentials');
  fs.writeFileSync(key, '[default]\naws_access_key_id = fake-id\naws_secret_access_key = fake-secret\n');
  const run = () => spawnSync(process.execPath, ['scripts/aws-emulator/check-artifacts.cjs', out, key], {encoding: 'utf8'});
  const file = path.join(out, 'create.json');
  fs.writeFileSync(file, '{}');
  assert.equal(run().status, 0);
  fs.writeFileSync(file, 'fake-secret');
  const result = run();
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Credential material/);
  assert.ok(!result.stderr.includes('fake-secret'));
});

test('snapshot preserves attributes across unordered paths and rejects state or failed plans', t => {
  const dir = directory(t);
  const file = path.join(dir, 'plan.json');
  const expected = JSON.parse(fs.readFileSync('examples/aws-emulator/containment/create.json'));
  const run = plan => {
    fs.writeFileSync(file, JSON.stringify(plan));
    return spawnSync(process.execPath, ['scripts/aws-emulator/create-snapshot.cjs', file], {encoding: 'utf8'});
  };
  const plan = {...expected, relevant_attributes: [...expected.relevant_attributes].reverse(), timestamp: '2026-01-01T00:00:00Z'};
  const result = run(plan);
  assert.equal(result.status, 0);
  assert.deepEqual(JSON.parse(result.stdout), expected);
  assert.notEqual(run({...plan, errored: true}).status, 0);
  assert.notEqual(run({terraform_version: '1.9.8', values: {}}).status, 0);
});
