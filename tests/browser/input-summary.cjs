const assert = require('node:assert/strict');
const fs = require('node:fs');
const {Worker, isMainThread, parentPort, workerData} = require('node:worker_threads');

const directory = isMainThread ? process.argv[2] || 'output/wasm-engine' : workerData.directory;
function load(mib = 512) {
  const source = fs.readFileSync(`${directory}/planorama.js`, 'utf8');
  const binding = new Function(source + '\nreturn wasm_bindgen;')();
  const wasm = binding.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: mib});
  return {binding, wasm};
}
const plan = JSON.stringify({format_version:'1.2', resource_changes:[
  {address:'aws_instance.a', type:'aws_instance', provider_name:'hashicorp/aws', change:{actions:['create', 'delete'], after:{secret:'PRIVATE'}}},
  {address:'aws_instance.a', type:'aws_instance', deposed:'old', change:{actions:['delete']}},
  {address:'data.aws_region.a', type:'aws_region', mode:'data', change:{actions:['read']}},
]});
const check = (code, index) => error => {
  const value = JSON.parse(String(error));
  assert.equal(value.code, code);
  assert.equal(value.input_index, index);
  assert(!String(error).includes('PRIVATE'));
  return true;
};

if (!isMainThread) {
  const {binding} = load();
  if (workerData.cancel) {
    parentPort.postMessage('ready');
    for (;;) binding.inspect_input_summary(plan);
  } else {
    parentPort.postMessage(JSON.parse(binding.inspect_named_input_summaries(JSON.stringify([{state:'a', json:plan}]))));
  }
} else {
  (async () => {
    const {binding} = load();
    const summary = JSON.parse(binding.inspect_input_summary(plan));
    assert.deepEqual(summary.counts, {
      resources:2, objects:3, current:2, deposed:1, managed:2, data:1,
      actions:{unchanged:0, create:0, update:0, delete:1, replacement:1, read:1, other:0},
    });
    assert.deepEqual(summary.providers, [
      {source:'registry.terraform.io/hashicorp/aws', evidence:'explicit', source_unavailable:false, objects:1},
      {source:'registry.terraform.io/hashicorp/aws', evidence:'inferred', source_unavailable:false, objects:2},
    ]);
    assert(!JSON.stringify(summary).includes('PRIVATE'));
    const named = JSON.parse(binding.inspect_named_input_summaries(JSON.stringify([{state:'a', json:plan}, {state:'b', json:plan}])));
    assert.deepEqual(named.inputs[0].summary, summary);
    assert.deepEqual(named.inputs[1].summary, summary);
    assert.equal(named.aggregate.counts.resources, 4);
    assert.equal(named.aggregate.counts.objects, 6);
    assert.throws(() => binding.inspect_named_input_summaries(JSON.stringify([{state:'a', json:plan}, {state:'b', json:'PRIVATE{'}])), check('invalid_json', 1));
    assert.throws(() => binding.inspect_named_input_summaries('PRIVATE{'), check('invalid_json'));
    assert.throws(() => binding.inspect_input_summary('{}'), check('unsupported_input'));
    assert.throws(() => binding.inspect_input_summary(' '.repeat(32 * 1024 * 1024 + 1)), check('input_too_large'));
    const structures = JSON.stringify({format_version:'1.2', resource_changes:[], extra:Array(32768).fill(0)});
    assert.throws(() => binding.inspect_input_summary(structures), check('resource_limit'));
    const large = JSON.stringify({format_version:'1.2', resource_changes:[], extra:'x'.repeat(8 * 1024 * 1024)});
    assert.throws(() => binding.inspect_input_summary(large), check('memory_limit'));
    assert.throws(() => binding.inspect_named_input_summaries(JSON.stringify([{state:'a', json:large}])), check('memory_limit', 0));
    const roomy = load(1024);
    assert.equal(JSON.parse(roomy.binding.inspect_input_summary(large)).counts.objects, 0);
    assert.throws(() => roomy.wasm.memory.grow(1024 * 16), RangeError);
    const rawSource = fs.readFileSync(`${directory}/planorama.js`, 'utf8').replace(/\(\(raw, capabilities\) => \{[\s\S]*$/, '');
    const raw = new Function(rawSource + '\nreturn wasm_bindgen;')();
    raw.initSync({module:fs.readFileSync(`${directory}/planorama_bg.wasm`), memory:new WebAssembly.Memory({initial:32, maximum:8192})});
    assert.throws(() => raw.inspect_input_summary(plan), check('memory_budget_mismatch'));
    assert.throws(() => raw.inspect_named_input_summaries(JSON.stringify([{state:'a', json:plan}])), check('memory_budget_mismatch'));
    const worker = new Worker(__filename, {workerData:{directory}});
    const result = await new Promise((resolve, reject) => { worker.once('message', resolve); worker.once('error', reject); });
    assert.deepEqual(result.inputs[0].summary, summary);
    await worker.terminate();
    const cancelled = new Worker(__filename, {workerData:{directory, cancel:true}});
    await new Promise((resolve, reject) => { cancelled.once('message', resolve); cancelled.once('error', reject); });
    assert.equal(await cancelled.terminate(), 1);
    console.log('Input summary: actions, providers, named rows, limits, imported memory and Worker cancellation passed');
  })().catch(error => { console.error(error); process.exitCode = 1; });
}
