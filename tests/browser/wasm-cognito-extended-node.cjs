const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown-roles', 'unknown-providers', 'parent', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cognito-extended-plan.json', 'utf8'));
  if (scenario === 'unknown-roles') plan.resource_changes[15].change.after_unknown = {roles: true};
  if (scenario === 'unknown-providers') plan.resource_changes[12].change.after_unknown = {supported_identity_providers: true};
  if (scenario === 'parent') plan.resource_changes[23].change.after_unknown = {user_pool_id: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_cognito_user_in_group.main'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    if (scenario === 'complete') assert.ok(result.content.includes('Identity pool roles'));
  }
}
console.log('Extended Cognito native/Wasm SVG and HTML parity passed');
