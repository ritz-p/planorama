const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const root = resolve(__dirname, '../..');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(root, 'target/release/planorama'));
for (const fixture of ['grafana', 'prometheus', 'prometheus-scraper']) {
  for (const scenario of ['complete', 'unknown', 'filtered']) {
    const input = JSON.parse(readFileSync(`tests/fixtures/${fixture}-plan.json`, 'utf8'));
    if (scenario === 'unknown') input.resource_changes.at(-1).change.after_unknown = fixture === 'prometheus-scraper' ? {scraper_id:true} : fixture === 'prometheus' ? {workspace_id:true} : {kms_key_id: true};
    const json = JSON.stringify(input);
    for (const format of ['svg', 'html']) {
      const options = {format, icon_mode: 'service-name'};
      const args = ['--format', format];
      if (scenario === 'filtered') {
        options.focus = input.resource_changes.at(-1).address;
        options.focus_depth = 0;
        args.push('--focus', options.focus, '--focus-depth', '0');
      }
      const result = JSON.parse(binding.generate(json, JSON.stringify(options)));
      assert.equal(result.content, native(json, args), `${fixture}/${scenario}/${format}`);
    }
  }
}
console.log('Observability native/Wasm SVG and HTML parity passed');
