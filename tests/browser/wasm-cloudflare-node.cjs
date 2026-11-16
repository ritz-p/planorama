const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const native = require('../support/native-reference.cjs')(resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama'));
const svgOnly = new Set([
  'foreign', 'dns-duplicate', 'applications-jurisdiction',
  'applications-domain-zone-unknown', 'applications-domain-zone-conflict',
  'applications-cname-unknown', 'applications-cname-literal',
  'applications-pages-service-unknown', 'applications-percentage-zero',
]);
const formatsFor = scenario => svgOnly.has(scenario) ? ['svg'] : ['svg', 'html'];
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
  for (const format of formatsFor(scenario)) {
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
  for (const format of formatsFor(scenario)) {
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
for (const scenario of ['applications', 'applications-unknown', 'applications-jurisdiction', 'applications-filtered', 'applications-lifecycle', 'applications-mixed', 'applications-domain-zone-unknown', 'applications-domain-zone-conflict', 'applications-cname-unknown', 'applications-cname-literal', 'applications-hyperdrive-sensitive', 'applications-pages-service-unknown', 'applications-percentage-sensitive', 'applications-percentage-zero']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-applications-plan.json', 'utf8'));
  if (scenario === 'applications-percentage-sensitive') plan.resource_changes[7].change.after_sensitive = {versions:[{percentage:true},{}]};
  if (scenario === 'applications-percentage-zero') {plan.resource_changes[7].change.after.versions[0].percentage=0;plan.resource_changes[7].change.after.versions[1].percentage=100;}
  if (scenario === 'applications-pages-service-unknown') plan.resource_changes[11].change.after_unknown = {deployment_configs: {production: {services: {BACKEND: {service: true}}}}};
  if (scenario === 'applications-hyperdrive-sensitive') plan.resource_changes[16].change.after_sensitive = {id: true};
  if (scenario === 'applications-cname-unknown') plan.resource_changes[15].change.after_unknown = {content: true};
  if (scenario === 'applications-cname-literal') plan.configuration.root_module.resources[15].expressions.content = {constant_value: 'site.pages.dev'};
  if (scenario === 'applications-domain-zone-unknown') plan.resource_changes[10].change.after_unknown = {zone_id: true};
  if (scenario === 'applications-domain-zone-conflict') plan.resource_changes[10].change.after.account_id = 'f'.repeat(32);
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
  for (const format of formatsFor(scenario)) {
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
for (const scenario of ['zero-trust', 'zero-trust-unknown', 'zero-trust-account', 'zero-trust-filtered', 'zero-trust-data', 'zero-trust-lifecycle', 'zero-trust-dns-unknown', 'zero-trust-dns-literal', 'zero-trust-waf-sensitive']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-zero-trust-plan.json', 'utf8'));
  if (scenario === 'zero-trust-waf-sensitive') plan.resource_changes[12].change.after_sensitive = {zone_id: true};
  if (scenario === 'zero-trust-dns-unknown') plan.resource_changes[11].change.after_unknown = {content: true};
  if (scenario === 'zero-trust-dns-literal') plan.configuration.root_module.resources[11].expressions.content = {constant_value: plan.resource_changes[11].change.after.content};
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
for (const scenario of require('../../scripts/cloudflare-capture.cjs').cases) {
  const input = readFileSync(`tests/fixtures/cloudflare-captured/${scenario}.json`, 'utf8');
  for (const format of ['svg', 'html']) {
    for (const changed of [false, true]) {
      const options = {format, icon_mode: 'service-name', changed_only: changed};
      const args = ['--format', format, '--icon-mode', 'service-name', ...(changed ? ['--changed-only'] : [])];
      const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
      assert.equal(result.content, native(input, args), `captured-${scenario}/${format}/changed=${changed}`);
      for (const payload of ['192.0.2.', 'fixture-only-not-a-real-token', 'export default', 'ZXhwb3J0IGRlZmF1bHQ']) {
        assert.ok(!result.content.includes(payload), payload);
      }
    }
  }
}
console.log('Cloudflare Provider-generated create/update/replacement/data native/Wasm parity passed');
for (const scenario of ['unknown', 'sensitive', 'scope', 'filtered', 'reordered']) {
  const plan = JSON.parse(readFileSync('tests/fixtures/cloudflare-captured/create.json', 'utf8'));
  const dns = plan.resource_changes.find(r => r.address === 'cloudflare_dns_record.www');
  if (scenario === 'sensitive') {
    dns.change.after.zone_id = 'PRIVATE_DIAGNOSTIC_VALUE';
    dns.change.after_sensitive = {zone_id: true};
  }
  if (scenario === 'scope') {
    const known = JSON.parse(readFileSync('tests/fixtures/cloudflare-captured/update.json', 'utf8'));
    Object.assign(plan, known);
    plan.resource_changes.find(r => r.type === 'cloudflare_workers_kv_namespace').change.after.account_id = 'e'.repeat(32);
  }
  if (scenario === 'reordered') {
    plan.resource_changes.reverse();
    plan.configuration.root_module.resources.reverse();
  }
  const input = JSON.stringify(plan);
  for (const format of ['svg', 'html']) {
    const options = {format, html_semantic_diagnostics: format === 'html'};
    const args = ['--format', format, ...(format === 'html' ? ['--html-semantic-diagnostics'] : [])];
    if (scenario === 'filtered') {
      options.focus = 'cloudflare_dns_record.www'; options.focus_depth = 0;
      args.push('--focus', options.focus, '--focus-depth', '0');
    }
    const result = JSON.parse(binding.generate(input, JSON.stringify(options)));
    assert.equal(result.content, native(input, args), `diagnostics-${scenario}/${format}`);
    assert.ok(!result.content.includes('PRIVATE_DIAGNOSTIC_VALUE'));
    const expected = scenario === 'sensitive' ? 'cloudflare_sensitive_value'
      : scenario === 'scope' ? 'cloudflare_scope_mismatch' : 'cloudflare_unknown_value';
    assert.ok(result.content.includes(expected), `${scenario}/${format}`);
  }
}
console.log('Cloudflare typed diagnostics native/Wasm parity passed');

for (const scenario of ['groups','groups-sensitive','groups-zone','groups-filtered']) {
 const plan=JSON.parse(readFileSync('tests/fixtures/cloudflare-zero-trust-plan.json','utf8'));
 if(scenario==='groups-sensitive') plan.resource_changes[3].change.after_sensitive={include:true};
 if(scenario==='groups-zone') plan.resource_changes[9].change.after.zone_id='b'.repeat(32);
 for(const format of ['svg','html']) {
  const options={format,html_semantic_diagnostics:format==='html'};
  const args=['--format',format,...(format==='html'?['--html-semantic-diagnostics']:[])];
  if(scenario==='groups-filtered'){options.focus='cloudflare_zero_trust_access_policy.staff';options.focus_depth=0;args.push('--focus',options.focus,'--focus-depth','0');}
  const input=JSON.stringify(plan);const result=JSON.parse(binding.generate(input,JSON.stringify(options)));
  assert.equal(result.content,native(input,args));assert.ok(!result.content.includes('PRIVATE_'));
  if(scenario==='groups-sensitive')assert.ok(result.content.includes('cloudflare_sensitive_value'));
  if(scenario==='groups-zone')assert.ok(result.content.includes('cloudflare_scope_mismatch'));
 }
}
console.log('Cloudflare Policy Group native/Wasm parity passed');
for (const scenario of ['services','services-only','service-environment','service-sensitive','service-filtered']) {
 const plan=JSON.parse(readFileSync('tests/fixtures/cloudflare-applications-plan.json','utf8'));
 if(scenario==='services-only')for(const i of [6,8]){
  plan.resource_changes[i].change.after.bindings=plan.resource_changes[i].change.after.bindings.filter(b=>b.type==='service');
  plan.configuration.root_module.resources[i].expressions.bindings.references=['cloudflare_worker.backend.name','cloudflare_workers_script.downstream.script_name'];
 }
 if(scenario==='service-environment')plan.resource_changes[6].change.after.bindings[4].environment='staging';
 if(scenario==='service-sensitive')plan.resource_changes[6].change.after_sensitive={bindings:[{},{},{},{},{service:true},{},{}, {}]};
 for(const format of ['svg','html']) {
  const options={format,html_semantic_diagnostics:format==='html'};
  const args=['--format',format,...(format==='html'?['--html-semantic-diagnostics']:[])];
  if(scenario==='service-filtered'){options.focus='cloudflare_worker_version.api';options.focus_depth=0;args.push('--focus',options.focus,'--focus-depth','0');}
  const input=JSON.stringify(plan),result=JSON.parse(binding.generate(input,JSON.stringify(options)));
  assert.equal(result.content,native(input,args));assert.ok(!result.content.includes('PRIVATE_'));
  if(scenario==='services'){assert.ok(result.content.includes('Service bindings'));assert.ok(result.content.includes('Storage bindings'));}
  if(scenario==='services-only'){
   const metadata=result.content.match(/<metadata id="resource-references">([\s\S]*?)<\/metadata>/)[1];
   const records=JSON.parse(metadata.replaceAll('&quot;','"').replaceAll('&amp;','&'));
   const rows=records.filter(r=>r.attribute==='bindings'&&r.complete);
   assert.equal(rows.length,2);assert.ok(rows.every(r=>r.targets.length===2));
  }
  if(scenario==='service-environment')assert.ok(result.content.includes('cloudflare_unsupported_contract'));
  if(scenario==='service-sensitive')assert.ok(result.content.includes('cloudflare_sensitive_value'));
 }
}
console.log('Cloudflare Service bindings native/Wasm parity passed');
for (const scenario of ['queues','queues-only','queue-sensitive','queue-http-pull','queue-filtered','queue-dlq-sensitive','queue-dlq-literal']) {
 const plan=JSON.parse(readFileSync('tests/fixtures/cloudflare-queues-plan.json','utf8'));
 const resource=address=>plan.resource_changes.find(r=>r.address===address);
 if(scenario==='queues-only')for(const address of ['cloudflare_workers_script.legacy','cloudflare_worker_version.backend']) {
  resource(address).change.after.bindings=resource(address).change.after.bindings.filter(b=>b.type==='queue');
  plan.configuration.root_module.resources.find(r=>r.address===address).expressions.bindings.references=['cloudflare_queue.jobs.queue_name','cloudflare_queue.audit.queue_name'];
 }
 if(scenario==='queue-dlq-sensitive')resource('cloudflare_queue_consumer.worker').change.after_sensitive={dead_letter_queue:true};
 if(scenario==='queue-dlq-literal')plan.configuration.root_module.resources.find(r=>r.address==='cloudflare_queue_consumer.worker').expressions.dead_letter_queue={constant_value:'audit'};
 if(scenario==='queue-sensitive')resource('cloudflare_queue.jobs').change.after_sensitive={queue_name:true,queue_id:true};
 if(scenario==='queue-http-pull')resource('cloudflare_queue_consumer.worker').change.after.type='http_pull';
 for(const format of ['svg','html']) {
  const options={format,html_semantic_diagnostics:format==='html'};
  const args=['--format',format,...(format==='html'?['--html-semantic-diagnostics']:[])];
  if(scenario==='queue-filtered'){options.focus='cloudflare_queue_consumer.worker';options.focus_depth=0;args.push('--focus',options.focus,'--focus-depth','0');}
  const input=JSON.stringify(plan),result=JSON.parse(binding.generate(input,JSON.stringify(options)));
  assert.equal(result.content,native(input,args),`${scenario}/${format}`);
  assert.ok(!result.content.includes('PRIVATE_'));
  if(scenario==='queues')for(const label of ['Queue producer bindings','Consumed Queue','Consumer Worker','Configured dead letter Queue','Storage bindings','Service bindings'])assert.ok(result.content.includes(label));
  if(scenario==='queue-sensitive')assert.ok(result.content.includes('cloudflare_sensitive_value'));
  if(scenario==='queue-http-pull')assert.ok(result.content.includes('cloudflare_unsupported_contract'));
  const metadata=result.content.match(/<metadata id="resource-references">([\s\S]*?)<\/metadata>/)[1];
  const rows=JSON.parse(metadata.replaceAll('&quot;','"').replaceAll('&amp;','&'));
  if(scenario==='queues-only'){
   const bindings=rows.filter(r=>r.attribute==='bindings'&&r.complete);
   assert.equal(bindings.length,2);assert.ok(bindings.every(r=>r.targets.length===2));
  }
  if(scenario==='queue-filtered'){
   const links=rows.filter(r=>['queue_id','script_name'].includes(r.attribute));
   assert.equal(links.length,2);assert.ok(links.every(r=>r.complete&&r.targets.length===1));
   assert.ok(result.content.includes('data-reference-status="unavailable"'));
  }
 }
}
console.log('Cloudflare Queues native/Wasm parity passed');

const queues=readFileSync('tests/fixtures/cloudflare-queues-plan.json','utf8');
for(const format of ['svg','html']){
 const result=JSON.parse(binding.generate_named(JSON.stringify(['first','second'].map(state=>({state,json:queues}))),JSON.stringify({format})));
 const expected=require('node:child_process').execFileSync(resolve(process.env.CARGO_TARGET_DIR||'target','release/planorama'),['--state','first=tests/fixtures/cloudflare-queues-plan.json','--state','second=tests/fixtures/cloudflare-queues-plan.json','--format',format,'-o','-'],{encoding:'utf8',maxBuffer:32*1024*1024});
 assert.equal(result.content,expected,`queues-named/${format}`);
 assert.ok(!result.content.includes('PRIVATE_'));
}
console.log('Cloudflare Queues named-state parity passed');
