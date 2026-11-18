const assert = require('node:assert/strict');
const {test} = require('node:test');
const {createHash} = require('node:crypto');
const {compare} = require('../../scripts/cloudflare-inventory-diff.cjs');

const provider = 'registry.terraform.io/cloudflare/cloudflare';
const type = 'cloudflare_example';
const text = {type: 'string', computed: true};
const fields = {
  account_id: {type: 'string', required: true},
  id: text,
  bindings: {optional: true, nested_type: {nesting_mode: 'list', attributes: {id: text}}},
};

function fixture() {
  const resource = {version: 0, block: {attributes: fields}};
  const value = {format_version: '1.0', provider_schemas: {[provider]: {
    resource_schemas: {[type]: resource}, data_source_schemas: {[type]: resource},
  }}};
  const pinned = Buffer.from(JSON.stringify(value));
  const inventory = {provider, version: '5.28.1', terraformVersion: '1.9.8',
    schemaSha256: createHash('sha256').update(pinned).digest('hex'),
    resources: ['managed', 'data'].map(mode => ({type, mode}))};
  const candidate = JSON.parse(pinned);
  const matrix = ['managed', 'data'].map(mode =>
    '| `cloudflare_example` | ' + mode + ' | Partial | Node | Logical | Example | ' +
    (mode === 'managed' ? '`bindings`: bindings' : '—') + ' | Account-scoped ID |').join('\n');
  const selection = {terraform_version: '1.9.8', provider_selections: {[provider]: '5.29.0'}};
  return {inventory, pinned, candidate, matrix, selection, schemas: candidate.provider_schemas[provider]};
}
function report(f) {
  return compare(f.inventory, f.pinned, Buffer.from(JSON.stringify(f.candidate)), f.selection, f.matrix);
}

test('reports per-mode classification, scope and nested reference changes with structural evidence', () => {
  const f = fixture();
  const attrs = f.schemas.resource_schemas[type].block.attributes;
  delete attrs.account_id;
  attrs.bindings.nested_type.nesting_mode = 'set';
  attrs.bindings.nested_type.attributes.id.sensitive = true;
  attrs.id = {type: 'number', optional: true};
  delete f.schemas.data_source_schemas[type];
  f.schemas.data_source_schemas.cloudflare_new = {version: 0, block: {block_types: {
    rule: {nesting_mode: 'list', block: {attributes: {id: text}}},
  }}};
  const result = report(f);
  assert.deepEqual(result.counts, {managed: {added: 0, removed: 0, changed: 1}, data: {added: 1, removed: 1, changed: 0}});
  const [removed, added, changed] = result.records;
  assert.deepEqual(removed.review, ['removed-type', 'endpoint-identity']);
  assert.equal(added.runtime, null);
  assert.equal(added.changes[0].after.block.block_types.rule.nesting_mode, 'list');
  assert.equal(changed.runtime.level, 'Partial');
  assert.deepEqual(changed.scope, {before: {account: ['account_id'], zone: []}, after: {account: [], zone: []}});
  assert.deepEqual(changed.review, ['configured-references', 'endpoint-identity', 'input-output-contract', 'potentially-breaking-shape', 'scope-path']);
  assert.deepEqual(changed.changes.find(c => c.path.endsWith('/nesting_mode')),
    {path: '/block/attributes/bindings/nested_type/nesting_mode', kind: 'changed', before: 'list', after: 'set'});
  assert.ok(changed.changes.some(c => c.path.endsWith('/attributes/id/sensitive') && c.after));
  assert.equal(result.candidate.version, '5.29.0');
});

test('ignores schema prose and key ordering but keeps user attributes named description', () => {
  const f = fixture();
  const block = f.schemas.resource_schemas[type].block;
  block.description = 'updated documentation';
  block.attributes = Object.fromEntries(Object.entries(block.attributes).reverse());
  assert.deepEqual(report(f).records, []);
  block.attributes.bindings.nested_type.attributes.description = {type: ['map', 'string'], required: true};
  const [change] = report(f).records;
  assert.deepEqual(change.changes.map(c => c.path), ['/block/attributes/bindings/nested_type/attributes/description']);
  assert.ok(change.review.includes('potentially-breaking-shape'));
});

test('rejects unverified baselines, missing modes, wrong providers and partial capability matrices', () => {
  for (const [mutate, message] of [
    [f => { f.inventory.schemaSha256 = 'wrong'; }, /fingerprint/],
    [f => { f.selection.provider_selections = {}; }, /selection/],
    [f => { f.selection.terraform_version = 'other'; }, /Terraform version/],
    [f => { delete f.schemas.data_source_schemas; }, /schema section/],
    [f => { f.candidate.provider_schemas = {}; }, /provider schema/],
    [f => { f.matrix = f.matrix.split('\n')[0]; }, /matrix types mismatch/],
    [f => { f.matrix += '\n' + f.matrix; }, /Duplicate/],
    [f => { f.schemas.resource_schemas[type].block.attributes.id = {}; }, /Incomplete attribute/],
  ]) {
    const f = fixture();
    mutate(f);
    assert.throws(() => report(f), message);
  }
});
