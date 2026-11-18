'use strict';
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
for (const [phase, action] of [['create', 'create'], ['refreshed', 'no-op'], ['destroy', 'delete']]) {
  const plan = JSON.parse(fs.readFileSync(path.join(process.argv[2], phase + '.json')));
  const resources = new Map(plan.resource_changes.map(r => [r.address, r.change]));
  const instance = resources.get('aws_instance.app');
  const subnet = resources.get('aws_subnet.private');
  const vpc = resources.get('aws_vpc.main');
  assert.equal(resources.size, 3);
  for (const change of [instance, subnet, vpc]) assert.deepEqual(change.actions, [action]);
  if (phase === 'create') {
    assert.equal(subnet.after_unknown.vpc_id, true);
    assert.equal(instance.after_unknown.subnet_id, true);
  } else {
    assert.ok(vpc.before.id && subnet.before.id);
    assert.equal(subnet.before.vpc_id, vpc.before.id);
    assert.equal(instance.before.subnet_id, subnet.before.id);
  }
}
