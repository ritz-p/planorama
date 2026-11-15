const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown', 'network', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/network-monitors-plan.json', 'utf8'));
  if (scenario === 'unknown') plan.resource_changes[4].change.after_unknown = {local_resource: true};
  if (scenario === 'network') plan.resource_changes[1].change.after_unknown = {vpc_id: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_networkmonitor_probe.main'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
  }
}
console.log('Network monitors native/Wasm SVG and HTML parity passed');
