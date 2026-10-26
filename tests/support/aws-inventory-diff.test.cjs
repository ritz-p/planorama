const {test} = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {compare} = require('../../scripts/aws-inventory-diff.cjs');

const matrix = '| `aws_example` | Node / Unknown | Logical / Spatial | Example | Regional | Basic | — |\n';
function fixture() {
  return {
    inventory: {version: '1.0.0', commit: 'a'.repeat(40), resources: [{type: 'aws_example', service: 'example', factory: 'resourceExample', region: 'Default', category: 'Example', serviceName: 'Example', documentation: 'example.md', deprecated: false, regionAttribute: false, dataSource: false}]},
    schema: {format_version: '1.0', provider_schemas: {'registry.terraform.io/hashicorp/aws': {
      resource_schemas: {aws_example: {version: 0, block: {attributes: {name: {type: 'string', required: true}, description: {type: 'string', optional: true}}, block_types: {child: {nesting_mode: 'list', block: {attributes: {id: {type: 'string', computed: true}}}}}}}},
      data_source_schemas: {}
    }}}
  };
}
const aws = fixture => fixture.schema.provider_schemas['registry.terraform.io/hashicorp/aws'];
function pack(value) {
  const bytes = Buffer.from(JSON.stringify(value.schema));
  return [{...value.inventory, schemaSha256: crypto.createHash('sha256').update(bytes).digest('hex')}, bytes];
}
function diff(old, next, support = matrix) {
  const [a, x] = pack(old), [b, y] = pack(next);
  return compare(a, b, x, y, support);
}

test('identical baseline has zero differences and stable provenance', () => {
  const report = diff(fixture(), fixture());
  assert.equal(report.schema.status, 'compared');
  assert.deepEqual(report.managed, {beforeCount: 1, afterCount: 1, added: [], removed: [], changed: []});
  assert.deepEqual(report.otherSchemas, {data_source_schemas: []});
  assert.deepEqual(report.review, []);
  assert.equal(JSON.stringify(report), JSON.stringify(diff(fixture(), fixture())));
});

test('add/remove are not inferred renames; runtime support is retained for review', () => {
  const old = fixture(), next = fixture();
  next.inventory.resources[0].type = 'aws_example_new';
  aws(next).resource_schemas.aws_example_new = aws(next).resource_schemas.aws_example;
  delete aws(next).resource_schemas.aws_example;
  aws(next).data_source_schemas.aws_data_only = {version: 0, block: {}};
  const report = diff(old, next);
  assert.deepEqual(report.managed.added, ['aws_example_new']);
  assert.deepEqual(report.managed.removed, ['aws_example']);
  assert.equal(report.managed.afterCount, 1);
  assert.match(report.review[0].reason, /retain historical/);
  assert.equal(report.review[0].runtime.scope, 'Regional');
  assert.equal(report.review[1].runtime, null);
  assert.equal(report.otherSchemas.data_source_schemas[0].path, '/aws_data_only');
});

test('metadata, alias, deprecation and nested structural flags are reviewed', () => {
  const old = fixture(), next = fixture();
  for (const value of [old, next]) {
    value.inventory.resources.push({...value.inventory.resources[0], type: 'aws_alias'});
    aws(value).resource_schemas.aws_alias = structuredClone(aws(value).resource_schemas.aws_example);
  }
  next.inventory.resources[1].aliasOf = 'aws_example';
  Object.assign(next.inventory.resources[0], {serviceName: 'New Example', deprecated: true});
  const block = aws(next).resource_schemas.aws_example.block;
  block.deprecated = true;
  block.attributes.name = {type: ['list', 'string'], optional: true, sensitive: true};
  delete block.attributes.description;
  block.attributes.extra = {nested_type: {nesting_mode: 'single', attributes: {value: {type: 'number', computed: true}}}};
  block.block_types.child.nesting_mode = 'set';
  block.block_types.child.block.attributes.id.type = 'number';
  const report = diff(old, next);
  assert.deepEqual(report.managed.changed.map(row => row.type), ['aws_alias', 'aws_example']);
  assert.equal(report.managed.changed[0].metadata[0].path, '/aliasOf');
  const paths = report.managed.changed[1].schema.changes.map(change => change.path);
  for (const path of ['/block/attributes/description', '/block/attributes/extra', '/block/attributes/name/type', '/block/attributes/name/required', '/block/attributes/name/optional', '/block/attributes/name/sensitive', '/block/block_types/child/nesting_mode', '/block/block_types/child/block/attributes/id/type', '/block/deprecated']) assert.ok(paths.includes(path), path);
  assert.equal(report.review.length, 2);
});

test('description prose is ignored but attributes called description are compared', () => {
  const old = fixture(), next = fixture();
  aws(next).resource_schemas.aws_example.block.description = 'Documentation only';
  assert.equal(diff(old, next).managed.changed.length, 0);
  aws(next).resource_schemas.aws_example.block.attributes.description.type = 'number';
  assert.equal(diff(old, next).managed.changed[0].schema.changes[0].path, '/block/attributes/description/type');
  aws(old).resource_schemas.aws_example.block.attributes.name.type = ['object', {description: 'string'}];
  aws(next).resource_schemas.aws_example.block.attributes.name.type = ['object', {description: 'number'}];
  assert.ok(diff(old, next).managed.changed[0].schema.changes.some(change => change.path === '/block/attributes/name/type'));
});

test('missing schemas are explicitly not compared, including changed metadata', () => {
  const [old, bytes] = pack(fixture());
  const next = structuredClone(old);
  next.resources[0].serviceName = 'Changed';
  const report = compare(old, next, bytes, undefined, matrix);
  assert.deepEqual(report.schema, {status: 'not-compared', missing: ['after']});
  assert.deepEqual(report.managed.changed[0].schema, {status: 'not-compared'});
  assert.equal(compare(old, old, undefined, undefined, matrix).schema.status, 'not-compared');
});

test('rejects mismatched hashes, incomplete inventories, schemas and unsupported formats', () => {
  const [baseline, bytes] = pack(fixture());
  assert.throws(() => compare(baseline, baseline, Buffer.from('{}'), bytes, matrix), /hash mismatch/);
  for (const field of ['version', 'commit', 'schemaSha256', 'resources']) {
    const invalid = {...baseline}; delete invalid[field];
    assert.throws(() => compare(invalid, baseline, undefined, bytes, matrix), /Incomplete inventory/);
  }
  assert.throws(() => compare({...baseline, formatVersion: 2}, baseline, bytes, bytes, matrix), /formatVersion/);
  for (const mutate of [
    f => { f.schema.format_version = '2.0'; },
    f => { delete f.schema.format_version; },
    f => { delete aws(f).data_source_schemas; },
    f => { delete aws(f).resource_schemas.aws_example; },
    f => { delete aws(f).resource_schemas.aws_example.block; },
    f => { aws(f).resource_schemas.aws_example.block.attributes.name = {}; },
    f => { f.inventory.resources.push(f.inventory.resources[0]); },
    f => { f.inventory.resources[0].aliasOf = 'aws_missing'; },
    f => { f.inventory.resources[0].deprecated = true; }
  ]) {
    const invalid = fixture(); mutate(invalid);
    assert.throws(() => diff(fixture(), invalid));
  }
  assert.throws(() => compare(baseline, baseline, bytes, bytes, ''), /support matrix/);
});

test('input object and resource ordering does not change report bytes', () => {
  const old = fixture(), next = fixture();
  next.inventory.resources.push({...next.inventory.resources[0], type: 'aws_second'});
  aws(next).resource_schemas.aws_second = structuredClone(aws(next).resource_schemas.aws_example);
  aws(next).resource_schemas.aws_example.block.attributes.name.sensitive = true;
  const expected = JSON.stringify(diff(old, next));
  next.inventory.resources.reverse();
  const reverse = value => Array.isArray(value) ? value.map(reverse) : value && typeof value === 'object' ? Object.fromEntries(Object.entries(value).reverse().map(([key, entry]) => [key, reverse(entry)])) : value;
  const actual = diff(old, {...next, schema: reverse(next.schema)});
  const original = JSON.parse(expected);
  original.after.schemaSha256 = actual.after.schemaSha256;
  assert.equal(JSON.stringify(actual), JSON.stringify(original));
});

test('CLI emits JSON, explicitly reports unavailable schemas and fails without partial reports', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'aws-diff-'));
  try {
    const [baseline, bytes] = pack(fixture());
    const inventoryPath = path.join(root, 'inventory.json');
    const schemaPath = path.join(root, 'schema.json');
    const matrixPath = path.join(root, 'matrix.md');
    fs.writeFileSync(inventoryPath, JSON.stringify(baseline));
    fs.writeFileSync(schemaPath, bytes);
    fs.writeFileSync(matrixPath, matrix);
    const run = (...args) => spawnSync(process.execPath, [path.resolve(__dirname, '../../scripts/aws-inventory-diff.cjs'), ...args], {encoding: 'utf8'});
    const complete = run(inventoryPath, inventoryPath, schemaPath, schemaPath, matrixPath);
    assert.equal(complete.status, 0, complete.stderr);
    assert.equal(JSON.parse(complete.stdout).schema.status, 'compared');
    const missing = run(inventoryPath, inventoryPath, '-', '-', matrixPath);
    assert.equal(missing.status, 0, missing.stderr);
    assert.equal(JSON.parse(missing.stdout).schema.status, 'not-compared');
    fs.writeFileSync(schemaPath, '{}');
    const invalid = run(inventoryPath, inventoryPath, schemaPath, schemaPath, matrixPath);
    assert.equal(invalid.status, 1);
    assert.equal(invalid.stdout, '');
    assert.match(invalid.stderr, /hash mismatch/);
    assert.equal(run().status, 1);
  } finally {
    fs.rmSync(root, {recursive: true, force: true});
  }
});
