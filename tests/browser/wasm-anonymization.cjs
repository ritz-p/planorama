const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const {execFileSync} = require('node:child_process');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
const input = readFileSync('tests/fixtures/anonymization-plan.json', 'utf8');
for (const format of ['svg', 'html']) {
  for (const named of [false, true]) {
    const options = JSON.stringify({anonymize: true, format});
    const inputs = JSON.stringify([{state: 'environment_sentinel', json: input}]);
    const result = JSON.parse(named ? binding.generate_named(inputs, options) : binding.generate(input, options));
    const args = named ? ['--state', 'environment_sentinel=-'] : ['-'];
    args.push('-o', '-', '--anonymize', '--format', format);
    assert.equal(result.content, execFileSync(binary, args, {input, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024}));
    const job = named ? binding.Generation.named(inputs, options) : new binding.Generation(input, options);
    try {
      let sliced;
      for (let steps = 0; steps < 10000 && !sliced; steps++) sliced = job.step(4096);
      assert.ok(sliced, 'resumable generation completes');
      assert.equal(JSON.parse(sliced).content, result.content);
    } finally { job.free(); }
    for (const secret of ['sentinel', '198.51.100.0/24', '2001:db8:abcd::/48', '739185026412', 'TOP_SECRET']) {
      assert.ok(!result.content.includes(secret), `${format}/${named}: ${secret}`);
    }
  }
}
console.log('Anonymization CLI/Wasm and resumable SVG/HTML parity passed');
const producer = readFileSync('tests/fixtures/multi/producer.json','utf8');
const consumer = readFileSync('tests/fixtures/multi/consumer.json','utf8');
for (const format of ['svg','html']) {
  const inputs = JSON.stringify([{state:'private_producer_sentinel',json:producer},{state:'private_consumer_sentinel',json:consumer}]);
  const options = JSON.stringify({format,anonymize:true,remote_state_mappings:[{consumer:'private_consumer_sentinel',remote:'data.terraform_remote_state.network',producer:'private_producer_sentinel'}]});
  const result = JSON.parse(binding.generate_named(inputs,options));
  const native = execFileSync(binary,['--state','private_producer_sentinel=tests/fixtures/multi/producer.json','--state','private_consumer_sentinel=tests/fixtures/multi/consumer.json','--remote-state','private_consumer_sentinel:data.terraform_remote_state.network=private_producer_sentinel','--anonymize','--format',format,'-o','-'],{encoding:'utf8',maxBuffer:8*1024*1024});
  assert.equal(result.content,native);
  assert.ok(!result.content.includes('sentinel'));
  assert.ok(result.content.includes('state_1'));
  assert.ok(result.content.includes('state_2'));
  const job=binding.Generation.named(inputs,options);
  try {
    let content;
    for(let steps=0;steps<10000&&!content;steps++) content=job.step(4096);
    assert.equal(JSON.parse(content).content,native);
  } finally {job.free();}
}
console.log('Anonymized cross-state mapping parity passed');
