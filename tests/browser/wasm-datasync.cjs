const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const binding = new Function(readFileSync('output/wasm-engine/planorama.js','utf8')+'\nreturn wasm_bindgen;')();
binding.initSync({module:readFileSync('output/wasm-engine/planorama_bg.wasm'),memory_budget_mib:512});
const native = require('../support/native-reference.cjs')('target/release/planorama');
for (const scenario of ['complete','unknown','filtered']) {
  const input = JSON.parse(readFileSync('tests/fixtures/datasync-plan.json','utf8'));
  if (scenario === 'unknown') input.resource_changes.filter(n=>n.type.startsWith('aws_datasync_')).forEach(n=>n.change.after_unknown=true);
  for (const format of ['svg','html']) {
    const options={format,icon_mode:'service-name'}, args=['--format',format];
    if (scenario === 'filtered') { options.focus='aws_datasync_location_efs.shared'; options.focus_depth=0; args.push('--focus',options.focus,'--focus-depth','0'); }
    const json=JSON.stringify(input);
    assert.equal(JSON.parse(binding.generate(json,JSON.stringify(options))).content,native(json,args),`${scenario}/${format}`);
  }
}
console.log('DataSync native/Wasm parity passed');
