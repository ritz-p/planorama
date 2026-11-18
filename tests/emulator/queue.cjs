'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const directory = process.argv[2];
for (const [phase, action] of Object.entries({
  create: 'create', refreshed: 'no-op', update: 'update', destroy: 'delete',
})) {
  const plan = JSON.parse(fs.readFileSync(path.join(directory, phase + '.json')));
  assert.equal(plan.terraform_version, '1.9.8');
  const [queue] = plan.resource_changes;
  assert.equal(plan.resource_changes.length, 1);
  assert.equal(queue.address, 'aws_sqs_queue.jobs');
  assert.deepEqual(queue.change.actions, [action], phase);
  if (phase === 'create') {
    assert.equal(queue.change.after_unknown.arn, true);
  } else {
    assert.match(queue.change.before.arn, /^arn:aws:sqs:us-east-1:\d{12}:planorama-emulator-jobs$/);
  }
  if (phase === 'update') {
    assert.equal(queue.change.before.delay_seconds, 0);
    assert.equal(queue.change.after.delay_seconds, 5);
  }
}
