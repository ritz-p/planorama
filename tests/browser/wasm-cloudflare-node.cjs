const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['foundation', 'sensitive', 'unknown', 'foreign', 'filtered', 'escaped']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-foundation-plan.json', 'utf8'));
  if (scenario === 'sensitive') plan.resource_changes[0].change.after_sensitive = true;
  if (scenario === 'unknown') plan.resource_changes[0].change.after_unknown = {account: true, id: true};
  if (scenario === 'foreign') plan.resource_changes[0].provider_name = 'example/cloudflare';
  if (scenario === 'escaped') plan.resource_changes[0].address = 'cloudflare_zone.example["<script>PRIVATE_ATTACK</script>"]';
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'filtered') {
      options.focus = 'cloudflare_dns_record.www'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_TOKEN'));
    assert.ok(!result.content.includes('PRIVATE_BODY'));
    assert.ok(!result.content.includes('<script>PRIVATE_ATTACK</script>'));
    if (scenario === 'foundation') assert.ok(result.content.includes('Cloudflare administration'));
  }
}
console.log('Cloudflare native/Wasm SVG and HTML parity passed');
