const assert = require('node:assert/strict');
const fs = require('node:fs');
const {Worker, isMainThread, parentPort, workerData} = require('node:worker_threads');
const {memoryLimits} = require('../../scripts/wasm-memory.cjs');

const directory = process.argv[2] || 'output/wasm-engine';
const invalid = code => error => JSON.parse(String(error)).code === code;
function load(directory, transform = source => source, runtime = WebAssembly) {
  const source = transform(fs.readFileSync(`${directory}/planorama.js`, 'utf8'));
  return new Function('WebAssembly', source + '\nreturn wasm_bindgen;')(runtime);
}

if (!isMainThread) {
  try {
    const binding = load(workerData.directory, source => source.replace(/\(\(raw, capabilities\) => \{[\s\S]*$/, ''));
    const bytes = fs.readFileSync(`${workerData.directory}/planorama_bg.wasm`);
    const {minimum} = memoryLimits(bytes);
    const memory = new WebAssembly.Memory({initial: minimum, maximum: minimum + 16});
    const wasm = binding.initSync({module: bytes, memory});
    binding.configure_memory_budget(512);
    if (workerData.mode === 'trap') wasm.__wbindgen_malloc(16 * 1024 * 1024, 1);
    const job = new binding.Generation(fs.readFileSync('tests/fixtures/terraform-plan.json', 'utf8'), '{"icon_mode":"service-name"}');
    for (let i = 0; i < 1_000_000; i++) {
      try {
        if (job.step(4096) !== undefined) throw new Error('Expected allocation failure');
      } catch (error) {
        if (workerData.mode !== 'step' || !invalid('memory_limit')(error)) throw error;
        assert.throws(() => job.step(1), invalid('generation_closed'));
        job.free();
        parentPort.postMessage('memory_limit');
        return;
      }
    }
    throw new Error('Expected trap');
  } catch (error) {
    if (workerData.mode !== 'trap' || !(error instanceof WebAssembly.RuntimeError)) throw error;
    parentPort.postMessage('trapped');
  }
} else {
  (async () => {
    const bytes = fs.readFileSync(`${directory}/planorama_bg.wasm`);
    const limits = memoryLimits(bytes);
    assert.equal(limits.maximum, 65536);
    const input = fs.readFileSync('tests/fixtures/viewer-plan.json', 'utf8');
    let reference = {};
    for (const mib of [undefined, 512, 1024, 2048, 4096]) {
      const binding = load(directory);
      const caps = JSON.parse(binding.memory_capabilities());
      assert.deepEqual(caps.allowed_mib, [512, 1024, 2048, 4096]);
      assert.equal(caps.maximum_bytes, 4294967296);
      assert.equal(caps.initial_pages, limits.minimum);
      const wasm = binding.initSync({module: bytes, memory_budget_mib: mib});
      assert.equal(wasm.memory.buffer.byteLength, limits.minimum * 65536);
      assert.throws(() => wasm.memory.grow((mib || 512) * 16), RangeError);
      const memoryBudget = mib === undefined ? {} : {memory_budget_mib: mib};
      for (const format of ['svg', 'html']) {
        const options = JSON.stringify({format, icon_mode: 'service-name', ...memoryBudget});
        const job = new binding.Generation(input, options);
        assert.equal(job.step(1), undefined);
        let result;
        while (result === undefined) result = job.step(4096);
        if (mib === undefined) reference[format] = result;
        assert.equal(result, reference[format]);
        assert.throws(() => job.step(1), invalid('generation_closed'));
        job.free();
        const cancelled = new binding.Generation(input, options);
        cancelled.step(1); cancelled.cancel();
        assert.throws(() => cancelled.step(1), invalid('generation_closed'));
        cancelled.free();
      }
      for (const value of ['null', 'true', '"512"', '[]', '{}', '0', '-512', '512.0', '512.5', '513', '8192', '4294967296']) {
        assert.throws(() => new binding.Generation(input, `{"memory_budget_mib":${value}}`), invalid('invalid_options'));
      }
      assert.throws(() => new binding.Generation(input, JSON.stringify({memory_budget_mib: mib === 1024 ? 512 : 1024})), invalid('memory_budget_mismatch'));
      assert.throws(() => binding.initSync({module: bytes, memory_budget_mib: mib === 1024 ? 512 : 1024}), invalid('memory_budget_mismatch'));
      assert.throws(() => binding.generate(input, '{"memory_budget_mib":512}'), invalid('invalid_options'));
      assert.throws(() => new binding.Generation(' '.repeat(2 * 1024 * 1024 + 1), JSON.stringify(memoryBudget)), invalid('resource_limit'));
    }
    for (const mib of [null, true, '512', [], {}, 0, -512, 512.5, 513, Infinity, NaN]) {
      assert.throws(() => load(directory).initSync({module: bytes, memory_budget_mib: mib}), invalid('invalid_options'));
    }
    const asynchronous = load(directory);
    await asynchronous({module_or_path: bytes, memory_budget_mib: 512});
    assert.equal(asynchronous.generate(input, '{"icon_mode":"service-name"}'), reference.svg);
    assert.throws(() => load(directory).initSync({module: bytes, memory: new WebAssembly.Memory({initial: 32})}), invalid('invalid_options'));
    let attempts = 0;
    const unavailable = load(directory, source => source, {...WebAssembly, Memory: class { constructor() { attempts++; throw new RangeError(); } }});
    assert.throws(() => unavailable.initSync({module: bytes, memory_budget_mib: 4096}), invalid('memory_unavailable'));
    assert.equal(attempts, 1, 'no fallback to a lower memory maximum');
    for (const mode of ['step', 'trap']) await new Promise((resolve, reject) => {
      const worker = new Worker(__filename, {workerData: {directory, mode}});
      const deadline = setTimeout(() => { worker.terminate(); reject(new Error('Trap test timed out')); }, 30_000);
      worker.once('message', message => { assert.equal(message, mode === 'trap' ? 'trapped' : 'memory_limit'); clearTimeout(deadline); worker.terminate().then(resolve, reject); });
      worker.once('error', error => { clearTimeout(deadline); reject(error); });
    });
    const retry = load(directory);
    retry.initSync({module: bytes});
    assert.equal(retry.generate(input, '{"icon_mode":"service-name"}'), reference.svg);
    console.log(`${directory}: memory budgets, output equality, cancellation, trap isolation and retry passed`);
  })().catch(error => { console.error(error); process.exitCode = 1; });
}
