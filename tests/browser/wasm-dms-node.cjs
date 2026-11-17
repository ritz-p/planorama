const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown', 'sensitive', 'scalar-array', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/dms-endpoints-plan.json', 'utf8'));
  if (scenario === 'unknown') plan.resource_changes[7].change.after_unknown = {engine_name: true};
  if (scenario === 'sensitive') plan.resource_changes[6].change.after_sensitive = {certificate_arn: true};
  if (scenario === 'scalar-array') plan.resource_changes[6].change.after.certificate_arn = [plan.resource_changes[6].change.after.certificate_arn];
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format};
    const args = ['--format', format];
    if (scenario === 'filtered') {
      options.focus = 'aws_dms_endpoint.source'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
  }
}
console.log('DMS native/Wasm SVG and HTML parity passed');
