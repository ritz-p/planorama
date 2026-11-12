const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['complete', 'unknown-revision', 'unknown-domain', 'filtered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/datazone-plan.json', 'utf8'));
  if (scenario === 'unknown-revision') plan.resource_changes[10].change.after_unknown = {revision: true};
  if (scenario === 'unknown-domain') plan.resource_changes[8].change.after_unknown = {domain_identifier: true};
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'aws_datazone_glossary_term.second'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    if (scenario === 'complete') assert.ok(result.content.includes('Revision-qualified asset forms'));
  }
}
console.log('DataZone catalog native/Wasm SVG and HTML parity passed');
