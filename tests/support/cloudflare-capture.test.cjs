const assert = require('node:assert/strict');
const {test} = require('node:test');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {check, seed, normalize, verifyPrepared} = require('../../scripts/cloudflare-capture.cjs');
const directory = path.resolve(__dirname, '../fixtures/cloudflare-captured');
const read = name => JSON.parse(fs.readFileSync(path.join(directory, `${name}.json`)));
const change = (plan, address) => plan.resource_changes.find(r => r.address === address).change;

test('normalization only removes timestamp and orders relevance without losing duplicates or other arrays', () => {
  const value = {
    timestamp: 'volatile',
    relevant_attributes: [{resource: 'b', attribute: ['id']}, {resource: 'a', attribute: ['name']}, {resource: 'b', attribute: ['id']}],
    resource_changes: [{address: 'b'}, {address: 'a'}],
    nested: {timestamp: 'retain', bindings: ['second', 'first']},
  };
  const expected = structuredClone(value);
  delete expected.timestamp;
  expected.relevant_attributes = [value.relevant_attributes[1], value.relevant_attributes[0], value.relevant_attributes[2]];
  assert.deepEqual(normalize(value), expected);
});

test('capture rejects changed prepared HCL, auto variables, extra configuration and prior state', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'cloudflare-prepared-test-'));
  try {
    fs.mkdirSync(path.join(temp, '.terraform'));
    for (const file of ['main.tf', '.terraform.lock.hcl']) fs.copyFileSync(path.join(directory, file), path.join(temp, file));
    verifyPrepared(temp, 'create');
    for (const file of ['extra.tf', 'override.tf.json', 'terraform.tfvars', 'extra.auto.tfvars.json', 'terraform.tfstate']) {
      fs.writeFileSync(path.join(temp, file), '{}');
      assert.throws(() => verifyPrepared(temp, 'create'), /unexpected prepared files/);
      fs.unlinkSync(path.join(temp, file));
    }
    fs.appendFileSync(path.join(temp, 'main.tf'), '\n');
    assert.throws(() => verifyPrepared(temp, 'create'), /prepared input changed/);
  } finally { fs.rmSync(temp, {recursive: true, force: true}); }
});

test('capture binds HCL, lock, generator, schema, artificial state and all plan bytes', () => {
  check();
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'cloudflare-capture-test-'));
  try {
    fs.cpSync(directory, temp, {recursive: true});
    for (const file of ['main.tf', 'data.tf', '.terraform.lock.hcl', 'schema.json', 'seed.json', 'create.json', 'update.json', 'replace.json', 'data.json']) {
      const target = path.join(temp, file);
      const before = fs.readFileSync(target);
      fs.appendFileSync(target, '\n');
      assert.throws(() => check(temp), /capture differs/, file);
      fs.writeFileSync(target, before);
    }
  } finally { fs.rmSync(temp, {recursive: true, force: true}); }
});

test('create unknowns and provider object/list expression provenance are preserved', () => {
  const plan = read('create');
  assert.equal(plan.resource_changes.length, 7);
  assert.ok(plan.resource_changes.every(r => r.change.actions.join() === 'create'));
  const dns = change(plan, 'cloudflare_dns_record.www');
  assert.equal(dns.after_unknown.zone_id, true);
  const version = change(plan, 'cloudflare_worker_version.api');
  assert.equal(version.after_unknown.worker_id, true);
  assert.equal(version.after_unknown.bindings[0].namespace_id, true);
  assert.equal(version.after_unknown.bindings[2].id, true);
  assert.ok(version.after_unknown.bindings.every(row => row.database_id === true));
  const refs = plan.configuration.root_module.resources.find(r => r.type === 'cloudflare_worker_version').expressions.bindings.references;
  assert.deepEqual(refs, [
    'cloudflare_workers_kv_namespace.cache.id', 'cloudflare_workers_kv_namespace.cache',
    'cloudflare_r2_bucket.assets.name', 'cloudflare_r2_bucket.assets',
    'cloudflare_d1_database.content.id', 'cloudflare_d1_database.content',
  ]);
});

test('artificial state yields real update and requested replacement plans', () => {
  assert.deepEqual(read('seed'), seed(read('create'), read('schema')));
  const update = read('update');
  for (const address of ['cloudflare_dns_record.www', 'cloudflare_workers_kv_namespace.cache']) {
    assert.deepEqual(change(update, address).actions, ['update']);
  }
  assert.equal(change(update, 'cloudflare_dns_record.www').before.ttl, 300);
  assert.equal(change(update, 'cloudflare_dns_record.www').after.ttl, 600);
  const kv = read('replace').resource_changes.find(r => r.type === 'cloudflare_workers_kv_namespace');
  assert.deepEqual(kv.change.actions, ['delete', 'create']);
  assert.equal(kv.action_reason, 'replace_by_request');
  assert.equal(kv.change.before.id, 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee');
  assert.equal(kv.change.after_unknown.id, true);
  assert.equal(change(read('replace'), 'cloudflare_worker_version.api').after_unknown.bindings[0].namespace_id, true);
});

test('known HTTP data read is only in prior values; unknown lookup is deferred', () => {
  const plan = read('data');
  assert.ok(!plan.resource_changes.some(r => r.address === 'data.cloudflare_zone.existing'));
  assert.ok(!plan.planned_values.root_module.resources.some(r => r.address === 'data.cloudflare_zone.existing'));
  const existing = plan.prior_state.values.root_module.resources.find(r => r.address === 'data.cloudflare_zone.existing');
  assert.equal(existing.mode, 'data');
  assert.equal(existing.values.id, 'cccccccccccccccccccccccccccccccc');
  assert.equal(existing.values.zone_id, existing.values.id);
  const deferred = plan.resource_changes.find(r => r.address === 'data.cloudflare_zone.deferred');
  assert.deepEqual(deferred.change.actions, ['read']);
  assert.equal(deferred.action_reason, 'read_because_config_unknown');
  assert.equal(deferred.change.after_unknown.id, true);
  assert.equal(change(plan, 'cloudflare_dns_record.deferred').after_unknown.zone_id, true);
});

test('captured schema attributes agree with the pinned managed/data inventory', () => {
  const inventory = JSON.parse(fs.readFileSync(path.resolve(directory, '../cloudflare-inventory/5.28.1.json')));
  const schema = read('schema');
  for (const [field, mode] of [['resource_schemas', 'managed'], ['data_source_schemas', 'data']]) {
    for (const [type, value] of Object.entries(schema[field])) {
      const entry = inventory.resources.find(r => r.type === type && r.mode === mode);
      assert.ok(entry, `${type}/${mode}`);
      assert.deepEqual(value.block.attributes, entry.attributes, `${type}/${mode}`);
    }
  }
});
