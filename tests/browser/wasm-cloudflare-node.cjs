const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
for (const scenario of ['foundation', 'sensitive', 'unknown', 'foreign', 'filtered', 'escaped', 'prior-sensitive']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-foundation-plan.json', 'utf8'));
  if (scenario === 'sensitive') plan.resource_changes[0].change.after_sensitive = true;
  if (scenario === 'unknown') plan.resource_changes[0].change.after_unknown = {account: true, id: true};
  if (scenario === 'foreign') plan.resource_changes[0].provider_name = 'example/cloudflare';
  if (scenario === 'escaped') plan.resource_changes[0].address = 'cloudflare_zone.example["<script>PRIVATE_ATTACK</script>"]';
  if (scenario === 'prior-sensitive') {
    plan.resource_changes = [];
    const resource = {address: 'data.cloudflare_dns_record.cached', mode: 'data', type: 'cloudflare_dns_record', name: 'cached'};
    plan.configuration = {root_module: {resources: [resource]}};
    plan.prior_state = {values: {root_module: {resources: [{...resource, provider_name: 'registry.terraform.io/cloudflare/cloudflare', values: {zone_id: '0123456789abcdef0123456789abcdef'}, sensitive_values: {zone_id: true}}]}}};
  }
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
    if (scenario === 'prior-sensitive') assert.ok(!result.content.includes('0123456789abcdef0123456789abcdef'));
    assert.ok(!result.content.includes('PRIVATE_BODY'));
    assert.ok(!result.content.includes('<script>PRIVATE_ATTACK</script>'));
    if (scenario === 'foundation') assert.ok(result.content.includes('Cloudflare administration'));
  }
}
console.log('Cloudflare native/Wasm SVG and HTML parity passed');
for (const scenario of ['dns', 'dns-unknown', 'dns-duplicate', 'dns-filtered', 'dns-lifecycle']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-dns-plan.json', 'utf8'));
  if (scenario === 'dns-unknown') plan.resource_changes[1].change.after_unknown = {id: true};
  if (scenario === 'dns-duplicate') {
    const duplicate = structuredClone(plan.resource_changes[1]);
    duplicate.address = 'cloudflare_zone.duplicate';
    plan.resource_changes.push(duplicate);
  }
  if (scenario === 'dns-lifecycle') {
    plan.resource_changes[2].change.actions = ['delete', 'create'];
    plan.resource_changes[2].previous_address = 'cloudflare_dns_record.old';
    const old = structuredClone(plan.resource_changes[2]);
    old.deposed = 'old'; old.change.actions = ['delete']; old.change.after = null;
    plan.resource_changes.push(old);
  }
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'dns-filtered') {
      options.focus = 'cloudflare_dns_record.www'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    assert.ok(result.content.includes('Zone administration'));
  }
}
const dns = readFileSync('tests/fixtures/cloudflare-dns-plan.json', 'utf8');
for (const format of ['svg', 'html']) {
  const result = JSON.parse(binding.generate_named(JSON.stringify(['first', 'second'].map(state => ({state, json: dns}))), JSON.stringify({format, icon_mode: 'service-name'})));
  const expected = require('node:child_process').execFileSync(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'), ['--state', 'first=tests/fixtures/cloudflare-dns-plan.json', '--state', 'second=tests/fixtures/cloudflare-dns-plan.json', '--format', format, '--icon-mode', 'service-name', '-o', '-'], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
  assert.equal(result.content, expected, `dns-named/${format}`);
}
console.log('Cloudflare DNS native/Wasm lifecycle, filtering and named-state parity passed');
for (const scenario of ['edge', 'edge-unknown', 'edge-account', 'edge-filtered', 'edge-data']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-edge-plan.json', 'utf8'));
  if (scenario === 'edge-unknown') plan.resource_changes[7].change.after_unknown = {default_pools: [false, true]};
  if (scenario === 'edge-account') plan.resource_changes[5].change.after.account_id = 'e'.repeat(32);
  if (scenario === 'edge-data') plan.resource_changes[5].mode = 'data';
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'edge-filtered') {
      options.focus = 'cloudflare_load_balancer.public'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
    assert.ok(result.content.includes('Default pools'));
  }
}
console.log('Cloudflare edge native/Wasm SVG and HTML parity passed');
for (const scenario of ['applications', 'applications-unknown', 'applications-jurisdiction', 'applications-filtered', 'applications-lifecycle', 'applications-mixed']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-applications-plan.json', 'utf8'));
  if (scenario === 'applications-unknown') plan.resource_changes[6].change.after_unknown = {bindings: true};
  if (scenario === 'applications-jurisdiction') plan.resource_changes[2].change.after.jurisdiction = 'default';
  if (scenario === 'applications-lifecycle') {
    plan.resource_changes[6].change.actions = ['delete', 'create'];
    const old = structuredClone(plan.resource_changes[2]);
    old.deposed = 'old'; old.change.actions = ['delete']; old.change.before = old.change.after; old.change.after = null;
    plan.resource_changes.push(old);
  }
  if (scenario === 'applications-mixed') {
    const aws = JSON.parse(readFileSync('tests/fixtures/cloudflare-foundation-plan.json', 'utf8')).resource_changes.filter(r => r.provider_name === 'registry.terraform.io/hashicorp/aws');
    assert.ok(aws.length > 0); plan.resource_changes.push(...aws);
  }
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'applications-filtered') {
      options.focus = 'cloudflare_worker.api'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_'));
  }
}
const applications = readFileSync('tests/fixtures/cloudflare-applications-plan.json', 'utf8');
for (const format of ['svg', 'html']) {
  const result = JSON.parse(binding.generate_named(JSON.stringify(['first', 'second'].map(state => ({state, json: applications}))), JSON.stringify({format, icon_mode: 'service-name'})));
  const expected = require('node:child_process').execFileSync(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'), ['--state', 'first=tests/fixtures/cloudflare-applications-plan.json', '--state', 'second=tests/fixtures/cloudflare-applications-plan.json', '--format', format, '--icon-mode', 'service-name', '-o', '-'], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
  assert.equal(result.content, expected, `applications-named/${format}`);
}
console.log('Cloudflare applications native/Wasm SVG, HTML and named-state parity passed');
for (const scenario of ['zero-trust', 'zero-trust-unknown', 'zero-trust-account', 'zero-trust-filtered', 'zero-trust-data', 'zero-trust-lifecycle']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-zero-trust-plan.json', 'utf8'));
  if (scenario === 'zero-trust-unknown') plan.resource_changes[4].change.after_unknown = {allowed_idps: true};
  if (scenario === 'zero-trust-account') plan.resource_changes[5].change.after.account_id = 'e'.repeat(32);
  if (scenario === 'zero-trust-data') plan.resource_changes[3].mode = 'data';
  if (scenario === 'zero-trust-lifecycle') {
    plan.resource_changes[4].change.actions = ['delete', 'create'];
    const old = structuredClone(plan.resource_changes[2]);
    old.deposed = 'old'; old.change.actions = ['delete']; old.change.before = old.change.after; old.change.after = null;
    plan.resource_changes.push(old);
  }
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, icon_mode: 'service-name'};
    const args = ['--format', format, '--icon-mode', 'service-name'];
    if (scenario === 'zero-trust-filtered') {
      options.focus = 'cloudflare_zero_trust_access_application.portal'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `${scenario}/${format}`);
    for (const privateValue of ['PRIVATE_', '192.0.2.', 'portal.example.invalid']) assert.ok(!result.content.includes(privateValue));
  }
}
console.log('Cloudflare Zero Trust native/Wasm SVG and HTML parity passed');
