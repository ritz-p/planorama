const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'null-target', 'unknown-target', 'computed-target', 'foreign-domain', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/datazone-environments-plan.json', 'utf8'));
  if (scenario === 'unknown-target') plan.resource_changes[9].change.after_unknown = {account_region: true};
  if (scenario === 'computed-target') delete plan.configuration.root_module.resources[9].expressions.account_identifier;
  if (scenario === 'foreign-domain') plan.resource_changes[8].change.after_unknown = {domain_identifier: true};
  if (scenario === 'null-target') plan.configuration.root_module.resources[9].expressions.account_identifier = {constant_value: null};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_datazone_environment.main'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    assert.ok(!result.content.includes('456789012345'));
    if (['unknown-target', 'computed-target', 'null-target'].includes(scenario)) assert.ok(!result.content.includes('345678901234'));
    if (scenario === 'complete') assert.ok(result.content.includes('Configured deployment target:'));
  }
}
console.log('DataZone environments native/Wasm SVG and HTML parity passed');
