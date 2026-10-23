const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const {Worker, isMainThread, parentPort, workerData} = require('node:worker_threads');
const {createHash} = require('node:crypto');
const os = require('node:os');

if (isMainThread) {
  const run = data => new Promise((resolve, reject) => {
    const worker = new Worker(__filename, {workerData: data});
    const timeout = setTimeout(() => {
      worker.terminate();
      reject(new Error('External measurement deadline exceeded'));
    }, 180_000);
    worker.once('message', result => { clearTimeout(timeout); resolve(result); });
    worker.once('error', error => { clearTimeout(timeout); reject(error); });
    worker.once('exit', code => { clearTimeout(timeout); if (code) reject(new Error(`Worker exited ${code}`)); });
  });
  (async () => {
    const results = [];
    const fixtures = process.env.PLANORAMA_STRESS === '1' ? ['viewer-plan', 'terraform-plan'] : ['viewer-plan'];
    for (const fixture of fixtures) {
      for (const format of ['svg', 'html']) {
        const reference = await run({fixture, format, mode: 'sync'});
        const sliced = await run({fixture, format, mode: 'step'});
        assert.equal(sliced.hash, reference.hash);
        assert.ok(sliced.steps > 1);
        results.push({fixture, format, reference, sliced});
      }
    }
    const files = Object.fromEntries(['planorama.js', 'planorama_bg.wasm'].map(file => [file, createHash('sha256').update(fs.readFileSync(`output/wasm-engine/${file}`)).digest('hex')]));
    console.log(JSON.stringify({runtime: process.version, platform: process.platform, cpu: os.cpus()[0]?.model, systemMemory: os.totalmem(), files, results}, null, 2));
  })().catch(error => { console.error(error); process.exitCode = 1; });
} else {
  (async () => {
    const directory = workerData.directory || 'output/wasm-engine';
    const binding = vm.runInThisContext(fs.readFileSync(`${directory}/planorama.js`, 'utf8') + '\nwasm_bindgen;');
    const wasm = binding.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`)});
    const input = fs.readFileSync(`tests/fixtures/${workerData.fixture}.json`, 'utf8');
    const options = JSON.stringify({format: workerData.format, icon_mode: 'service-name'});
    const invalid = code => value => JSON.parse(String(value)).code === code;
    assert.throws(() => new binding.Generation(input, ' '.repeat(65537)), invalid('resource_limit'));
    new binding.Generation(input, options.padEnd(65536, ' ')).free();
    assert.throws(() => new binding.Generation(input, JSON.stringify({modules: Array(257).fill('module.a')})), invalid('resource_limit'));
    const checked = new binding.Generation(input, options);
    const untouched = new binding.Generation(input, options);
    try {
      for (const units of [1.5, 4294967297, -4294967295, NaN, Infinity, -Infinity, 0, 4097]) {
        assert.throws(() => checked.step(units), invalid('invalid_options'));
      }
      assert.equal(checked.step(1), untouched.step(1));
      let actual, expected;
      do {
        actual = checked.step(4096);
        expected = untouched.step(4096);
        assert.equal(actual, expected);
      } while (actual === undefined);
    } finally { checked.free(); untouched.free(); }
    const started = performance.now();
    const cpu = process.cpuUsage();
    let result, steps = 0, longest = 0, activeMs = 0, memory = wasm.memory.buffer.byteLength;
    if (workerData.mode === 'sync') {
      result = binding.generate(input, options);
      activeMs = longest = performance.now() - started;
    } else {
      const job = new binding.Generation(input, options);
      let units = 64;
      try {
        while (result === undefined) {
          const start = performance.now();
          result = job.step(units);
          const elapsed = performance.now() - start;
          longest = Math.max(longest, elapsed);
          activeMs += elapsed;
          steps++;
          memory = Math.max(memory, wasm.memory.buffer.byteLength);
          units = Math.max(1, Math.min(4096, units * 2, Math.floor(units * 4 / Math.max(elapsed, 0.01))));
          if (result === undefined) await new Promise(resolve => setTimeout(resolve, Math.max(25, elapsed * 9)));
        }
        assert.throws(() => job.step(1), /generation_closed/);
      } finally {
        job.free();
      }
    }
    memory = Math.max(memory, wasm.memory.buffer.byteLength);
    const elapsedMs = performance.now() - started;
    const usage = process.cpuUsage(cpu);
    parentPort.postMessage({
      hash: createHash('sha256').update(result).digest('hex'),
      inputBytes: Buffer.byteLength(input), outputBytes: Buffer.byteLength(result),
      steps, longestMs: longest, activeMs, elapsedMs,
      processCpuMs: (usage.user + usage.system) / 1000,
      wasmBytes: memory, processRssBytes: process.memoryUsage().rss,
    });
  })().catch(error => { throw error; });
}
