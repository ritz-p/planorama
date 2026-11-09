const { test } = require('node:test');
const assert = require('node:assert/strict');
const { normalizeProvider } = require('../../scripts/compatibility-provider.cjs');
const capture = require('../fixtures/compatibility/1.9.8/aws670/plan.json');

test('provider comparison rejects unexpected IPAM defaults and unknown changes', () => {
  for (const mutate of [
    plan => { plan.planned_values.root_module.child_modules[0].resources.find(r => r.type === 'aws_subnet').values.ipv4_ipam_pool_id = 'unexpected'; },
    plan => { plan.resource_changes.find(r => r.type === 'aws_lb').change.after_unknown.enable_prefix_for_ipv6_source_nat = false; },
  ]) {
    const plan = structuredClone(capture);
    mutate(plan);
    assert.throws(() => normalizeProvider(plan), assert.AssertionError);
  }
});
