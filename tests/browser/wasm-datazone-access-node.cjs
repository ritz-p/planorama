const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown-user-type', 'ambiguous-principal', 'unknown-domain', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/datazone-access-plan.json', 'utf8'));
  if (scenario === 'unknown-user-type') plan.resource_changes[7].change.after_unknown = {user_type: true};
  if (scenario === 'ambiguous-principal') plan.resource_changes[11].change.after.principal[0].user[0].all_users_grant_filter = [{}];
  if (scenario === 'unknown-domain') plan.resource_changes[6].change.after_unknown = {domain_id: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_datazone_policy_grant.profile'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    if (scenario === 'complete') assert.ok(result.content.includes('Domain-qualified grant entity'));
  }
}
console.log('DataZone access native/Wasm SVG and HTML parity passed');
