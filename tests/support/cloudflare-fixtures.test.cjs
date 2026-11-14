const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const crypto = require('node:crypto');
const {fixtures, outputs} = require('../../scripts/cloudflare-fixtures.cjs');
const inventory = JSON.parse(fs.readFileSync('tests/fixtures/cloudflare-inventory/5.28.1.json'));
const schemas = new Map(inventory.resources.filter(r => r.mode === 'managed').map(r => [r.type, r.attributes]));
function object(value, path) {
  assert.ok(value !== null && typeof value === 'object' && !Array.isArray(value), path);
}
function type(value, shape, path) {
  if (value === null) return;
  if (typeof shape === 'string') {
    if (shape !== 'dynamic') assert.equal(typeof value, {string:'string', number:'number', bool:'boolean'}[shape], path);
    return;
  }
  const [kind, nested] = shape;
  if (kind === 'list' || kind === 'set' || kind === 'tuple') {
    assert.ok(Array.isArray(value), path);
    if (kind === 'tuple') assert.equal(value.length, nested.length, path);
    if (kind === 'set') assert.equal(new Set(value.map(v => JSON.stringify(v))).size, value.length, path);
    value.forEach((v,i) => type(v, kind === 'tuple' ? nested[i] : nested, `${path}[${i}]`));
  } else {
    object(value, path);
    assert.ok(kind === 'map' || kind === 'object', path);
    for (const [key, v] of Object.entries(value)) {
      if (kind === 'object') assert.ok(Object.hasOwn(nested, key), `${path}.${key}`);
      type(v, kind === 'map' ? nested : nested[key], `${path}.${key}`);
    }
  }
}
function attributes(value, schema, path) {
  object(value, path);
  for (const key of Object.keys(value)) assert.ok(Object.hasOwn(schema, key), `${path}.${key}: unknown attribute`);
  for (const [key, entry] of Object.entries(schema)) {
    const v = value[key], field = `${path}.${key}`;
    if (entry.required) assert.ok(v !== undefined && v !== null, `${field}: required`);
    if (v === undefined || v === null) continue;
    if (entry.type) type(v, entry.type, field);
    if (entry.nested_type) {
      const {nesting_mode: mode, attributes: nested} = entry.nested_type;
      let rows;
      if (mode === 'single') rows = [v];
      else if (mode === 'map') {object(v, field); rows = Object.values(v);}
      else {assert.ok(mode === 'list' || mode === 'set', field); assert.ok(Array.isArray(v), field); rows = v;}
      rows.forEach((row,i) => attributes(row, nested || {}, `${field}[${i}]`));
    }
  }
}
test('representative fixtures reproduce exact bytes and pinned synthetic provenance', () => {
  for (const [file, bytes] of outputs()) assert.equal(fs.readFileSync(file, 'utf8'), bytes, file);
  const provenance = JSON.parse(fs.readFileSync('tests/fixtures/cloudflare-provenance.json'));
  assert.equal(provenance.kind, 'synthetic');
  assert.equal(provenance.terraformVersion, '1.9.8');
  assert.equal(provenance.providerCommit, inventory.commit);
  assert.equal(provenance.schemaSha256, inventory.schemaSha256);
  assert.equal(provenance.fixtures.length, 4);
  for (const {file,sha256} of provenance.fixtures) assert.equal(crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'), sha256);
});
test('all four representative families match registered v5 schema shapes', () => {
  for (const [name, fixture] of fixtures()) {
    assert.equal(fixture.terraform_version, '1.9.8');
    for (const resource of fixture.resource_changes) {
      assert.equal(resource.provider_name, inventory.provider);
      assert.equal(resource.mode, 'managed');
      assert.ok(schemas.has(resource.type), resource.type);
      attributes(resource.change.after, schemas.get(resource.type), `${name}/${resource.address}`);
      const configuration = fixture.configuration.root_module.resources.find(r => r.address === resource.address);
      assert.ok(configuration, resource.address);
      for (const field of Object.keys(configuration.expressions)) {
        const schema = schemas.get(resource.type)[field];
        assert.ok(schema && (schema.required || schema.optional), `${resource.address}.${field}: configurable attribute`);
      }
    }
  }
});
test('schema validation rejects invented fields, required omissions and wrong collection shapes', () => {
  const source = fixtures().get('applications').resource_changes.find(r => r.type === 'cloudflare_worker_version');
  for (const mutate of [v => {delete v.worker_id;}, v => {v.bindings = {};}, v => {v.bindings[0].namespace_guess = 'fake';}, v => {v.bindings[1].namespace_id = 7;}]) {
    const v = structuredClone(source.change.after); mutate(v);
    assert.throws(() => attributes(v, schemas.get(source.type), source.address));
  }
});
test('Core shape evidence and adversarial inputs have distinct reproducible provenance', () => {
  const provenance = JSON.parse(fs.readFileSync('tests/fixtures/cloudflare-provenance.json'));
  const shape = JSON.parse(fs.readFileSync(provenance.coreExpressionShapeEvidence));
  assert.equal(shape.terraformVersion, provenance.terraformVersion);
  assert.equal(shape.sourceSha256, crypto.createHash('sha256').update(fs.readFileSync(shape.source)).digest('hex'));
  assert.deepEqual(shape.references, ['terraform_data.endpoint', 'terraform_data.endpoint.id']);
  assert.equal(shape.attribute, 'input');
  for (const source of provenance.adversarialFixtures) {
    assert.equal(source.kind, 'hand-authored-adversarial');
    assert.equal(source.sha256, crypto.createHash('sha256').update(fs.readFileSync(source.file)).digest('hex'));
  }
});
