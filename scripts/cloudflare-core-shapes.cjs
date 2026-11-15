const fs = require('node:fs');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
if (process.argv.length < 3 || process.argv.length > 4 || (process.argv[3] && process.argv[3] !== '--write')) throw new Error('Usage: node scripts/cloudflare-core-shapes.cjs PLAN_JSON [--write]');
const plan = JSON.parse(fs.readFileSync(process.argv[2]));
assert.equal(plan.terraform_version, '1.9.8');
const consumer = plan.configuration.root_module.resources.find(r => r.address === 'terraform_data.consumer');
assert.deepEqual(Object.keys(consumer.expressions.input), ['references']);
const references = [...new Set(consumer.expressions.input.references)].sort();
assert.deepEqual(references, ['terraform_data.endpoint', 'terraform_data.endpoint.id']);
const source = 'tests/fixtures/cloudflare-core-shapes/main.tf';
const capture = {
  kind: 'terraform-core-expression-shape', terraformVersion: plan.terraform_version,
  source, sourceSha256: crypto.createHash('sha256').update(fs.readFileSync(path.join(root, source))).digest('hex'),
  attribute: 'input', references,
  limits: 'Builtin terraform_data verifies object/list attribute expression shape, not Cloudflare API behavior or a captured Cloudflare deployment.'
};
const bytes = JSON.stringify(capture, null, 2) + '\n';
const target = path.join(root, 'tests/fixtures/cloudflare-core-shapes.json');
if (process.argv[3] === '--write') fs.writeFileSync(target, bytes);
else assert.equal(fs.readFileSync(target, 'utf8'), bytes);
console.log('Terraform Core object/list attribute references are aggregate and explicit');
