'use strict';

const fs = require('node:fs');
const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (plan.terraform_version !== '1.9.8' || plan.errored || !plan.complete ||
    !plan.resource_changes?.length || !plan.resource_changes.every(resource =>
      resource.change.actions.length === 1 && resource.change.actions[0] === 'create')) {
  throw new Error('Expected a complete Terraform create plan');
}
if (!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/.test(plan.timestamp)) {
  throw new Error('Expected the Terraform plan timestamp');
}
delete plan.timestamp;
process.stdout.write(JSON.stringify(plan, null, 2) + '\n');
