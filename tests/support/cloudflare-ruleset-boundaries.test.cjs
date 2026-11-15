const assert = require('node:assert/strict');
const {test} = require('node:test');
const inventory = require('../fixtures/cloudflare-inventory/5.28.1.json');

test('pinned Ruleset overrides and origins do not carry Pool or Worker identities', () => {
  const schema = inventory.resources.find(r => r.mode === 'managed' && r.type === 'cloudflare_ruleset');
  const parameters = schema.attributes.rules.nested_type.attributes.action_parameters.nested_type.attributes;
  assert.deepEqual(Object.keys(parameters.origin.nested_type.attributes).sort(), ['host', 'port']);
  assert.deepEqual(Object.keys(parameters.overrides.nested_type.attributes).sort(), ['action', 'categories', 'enabled', 'rules', 'sensitivity_level']);
  for (const field of ['pool', 'pool_id', 'worker', 'worker_id', 'service']) assert.equal(parameters[field], undefined);
  assert.equal(parameters.id.type, 'string');
  assert.equal(parameters.from_list.nested_type.attributes.name.type, 'string');
});
