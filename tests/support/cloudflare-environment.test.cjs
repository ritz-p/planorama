const assert = require('node:assert/strict');
const {test} = require('node:test');
const inventory = require('../fixtures/cloudflare-inventory/5.28.1.json');

test('pinned Worker endpoints cannot prove named Service Binding environments', () => {
  assert.equal(inventory.version, '5.28.1');
  for (const mode of ['managed', 'data']) {
    for (const type of ['cloudflare_worker', 'cloudflare_workers_script']) {
      const row = inventory.resources.find(r => r.mode === mode && r.type === type);
      assert.ok(row, `${mode}/${type}`);
      assert.equal(row.attributes.environment, undefined);
    }
  }
  assert.ok(!inventory.resources.some(r => /worker.*environment/.test(r.type)));
  for (const type of ['cloudflare_worker_version', 'cloudflare_workers_script']) {
    const row = inventory.resources.find(r => r.mode === 'managed' && r.type === type);
    for (const field of ['service', 'environment', 'entrypoint']) {
      assert.equal(row.attributes.bindings.nested_type.attributes[field].type, 'string');
    }
  }
});
