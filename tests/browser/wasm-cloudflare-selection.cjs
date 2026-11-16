const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
const fixture = 'tests/fixtures/cloudflare-administration-plan.json';
const json = readFileSync(fixture, 'utf8');
for (const format of ['svg', 'html']) for (const named of [false, true]) for (const anonymize of [false, true]) for (const unknown of [false, true]) {
  const account = unknown ? 'unknown' : 'a'.repeat(32);
  const options = JSON.stringify({format, cloudflare_accounts: [account], cloudflare_groups: true, anonymize});
  const inputs = JSON.stringify([{state: 'first', json}, {state: 'second', json}]);
  const result = JSON.parse(named ? binding.generate_named(inputs, options) : binding.generate(json, options));
  const args = [...(named ? ['--state', `first=${fixture}`, '--state', `second=${fixture}`] : [fixture]), '--cloudflare-account', account, '--cloudflare-groups', '--format', format, '-o', '-', ...(anonymize ? ['--anonymize'] : [])];
  assert.equal(result.content, execFileSync(binary, args, {encoding: 'utf8', maxBuffer: 16 * 1024 * 1024}));
  assert.ok(result.content.includes('data-cloudflare-management-member'));
  assert.ok(!result.content.includes('data-terraform-address="aws_s3_bucket.mixed"'));
  if (anonymize) for (const secret of ['a'.repeat(32), 'b'.repeat(32), 'deadbeef', 'cloudflare.other']) assert.ok(!result.content.includes(secret));
  const job = named ? binding.Generation.named(inputs, options) : new binding.Generation(json, options);
  try {
    let sliced;
    for (let index = 0; index < 10000 && !sliced; index++) sliced = job.step(4096);
    assert.ok(sliced);
    assert.equal(JSON.parse(sliced).content, result.content);
  } finally { job.free(); }
}
for (const cloudflare_zones of [null, true, ['example.invalid']]) assert.throws(() => binding.generate(json, JSON.stringify({cloudflare_zones})));
console.log('Cloudflare scope native/Wasm/resumable and anonymized parity passed');
