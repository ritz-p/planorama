const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
const fixture = 'tests/fixtures/cloudflare-imported-plan.json';
const json = readFileSync(fixture, 'utf8');
for (const format of ['svg', 'html']) for (const named of [false, true]) for (const anonymize of [false, true]) {
  const options = JSON.stringify({format, cloudflare_value_matches: true, anonymize});
  const inputs = JSON.stringify([{state: 'private_environment', json}]);
  const result = JSON.parse(named ? binding.generate_named(inputs, options) : binding.generate(json, options));
  const args = [...(named ? ['--state', `private_environment=${fixture}`] : [fixture]), '--cloudflare-value-matches', '--format', format, '-o', '-', ...(anonymize ? ['--anonymize'] : [])];
  assert.equal(result.content, execFileSync(binary, args, {encoding: 'utf8', maxBuffer: 16 * 1024 * 1024}));
  assert.ok(result.content.includes('data-evidence="configured-value"'));
  assert.ok(result.content.includes('typed-configured-value'));
  assert.ok(!result.content.includes('PRIVATE_IMPORT_ID'));
  const job = named ? binding.Generation.named(inputs, options) : new binding.Generation(json, options);
  try {
    let sliced;
    for (let index = 0; index < 10000 && !sliced; index++) sliced = job.step(4096);
    assert.ok(sliced);
    assert.equal(JSON.parse(sliced).content, result.content);
  } finally { job.free(); }
}
for (const cloudflare_value_matches of [null, 1, 'true']) {
  assert.throws(() => binding.generate(json, JSON.stringify({cloudflare_value_matches})));
}
console.log('Cloudflare configured-value native/Wasm/resumable and anonymized parity passed');
for (const missing of ['configuration', 'provider_config_key']) {
  const value = JSON.parse(json);
  if (missing === 'configuration') delete value.configuration;
  else for (const resource of value.configuration.root_module.resources) delete resource.provider_config_key;
  const input = JSON.stringify(value);
  const options = JSON.stringify({cloudflare_value_matches: true});
  const result = JSON.parse(binding.generate(input, options));
  assert.ok(!result.content.includes('data-evidence="configured-value"'));
  assert.equal(result.content, execFileSync(binary, ['-', '--cloudflare-value-matches', '-o', '-'], {input, encoding: 'utf8'}));
  const job = new binding.Generation(input, options);
  try {
    let sliced;
    for (let index = 0; index < 10000 && !sliced; index++) sliced = job.step(4096);
    assert.equal(JSON.parse(sliced).content, result.content);
  } finally { job.free(); }
}
console.log('Missing Cloudflare provider configuration remains unrecovered in native/Wasm/resumable generation');
