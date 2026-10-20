const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { readInputs, sourceInputs, digest, recreateInputs } =
  require('../../scripts/terraform-compatibility-inputs.cjs');

function workspace(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-compatibility-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

test('preparing again removes obsolete HCL, modules and prior state', t => {
  const root = workspace(t);
  const target = path.join(root, 'cases', 'example');
  recreateInputs(root, target, new Map([
    ['old.tf', 'resource "terraform_data" "old" {}'],
    ['old-module/main.tf', 'resource "terraform_data" "child" {}']
  ]));
  fs.writeFileSync(path.join(target, 'terraform.tfstate'), '{}');
  const inputs = new Map([['main.tf', 'resource "terraform_data" "current" {}']]);
  recreateInputs(root, target, inputs);
  assert.deepEqual(fs.readdirSync(target), ['main.tf']);
  assert.deepEqual(readInputs(target), inputs);
});

test('source digest tracks nested HCL, file deletion, extra HCL and local state', t => {
  const root = workspace(t);
  const source = path.join(root, 'source');
  const extra = path.join(root, 'extra.tf');
  fs.mkdirSync(path.join(source, 'module'), { recursive: true });
  fs.writeFileSync(path.join(source, 'main.tf'), 'terraform { required_version = "= 1.9.8" }\n');
  fs.writeFileSync(path.join(source, 'module/main.tf'), 'resource "terraform_data" "a" {}');
  fs.writeFileSync(path.join(source, 'shared-state.json'), '{"outputs":{}}');
  fs.writeFileSync(extra, 'check "fixture" {}');
  const fingerprint = () => digest(sourceInputs(source, extra, '1.16.6', null));
  const initial = fingerprint();
  fs.appendFileSync(path.join(source, 'module/main.tf'), '\nresource "terraform_data" "b" {}');
  assert.notEqual(fingerprint(), initial);
  const changed = fingerprint();
  fs.unlinkSync(path.join(source, 'module/main.tf'));
  assert.notEqual(fingerprint(), changed);
  const deleted = fingerprint();
  fs.writeFileSync(extra, 'check "different" {}');
  assert.notEqual(fingerprint(), deleted);
  const extraChanged = fingerprint();
  fs.writeFileSync(path.join(source, 'shared-state.json'), '{"outputs":{"id":"fixture"}}');
  assert.notEqual(fingerprint(), extraChanged);
});

test('source digest is stable across line endings and directory enumeration order', t => {
  const root = workspace(t);
  fs.writeFileSync(path.join(root, 'main.tf'), 'terraform {\r\n}\r\n');
  const windows = digest(readInputs(root));
  fs.writeFileSync(path.join(root, 'main.tf'), 'terraform {\n}\n');
  assert.equal(digest(readInputs(root)), windows);
  assert.equal(digest(new Map([['b.tf', 'b'], ['a.tf', 'a']])),
    digest(new Map([['a.tf', 'a'], ['b.tf', 'b']])));
});

test('preparation refuses deletion outside its dedicated workspace', t => {
  const root = workspace(t);
  const source = path.join(root, 'source');
  fs.mkdirSync(source);
  fs.writeFileSync(path.join(source, 'keep.tf'), 'keep');
  const work = path.join(root, 'work');
  fs.mkdirSync(work);
  assert.throws(() => recreateInputs(work, source, new Map()), /inside/);
  assert.throws(() => recreateInputs(work, work, new Map()), /inside/);
  assert.equal(fs.readFileSync(path.join(source, 'keep.tf'), 'utf8'), 'keep');
});
