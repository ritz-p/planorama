const assert = require('node:assert/strict');
const fs = require('node:fs');

if (process.env.PLANORAMA_MEMORY_STRESS !== '1') {
  console.log('Set PLANORAMA_MEMORY_STRESS=1 in an isolated container to test 2/4 GiB boundaries.');
} else {
  const directory = process.argv[2] || 'output/wasm-engine';
  let highPointer = false;
  const source = fs.readFileSync(`${directory}/planorama.js`, 'utf8')
    .replace('function getStringFromWasm0(ptr, len) {', 'function getStringFromWasm0(ptr, len) { observe(ptr >>> 0);');
  const binding = new Function('observe', source + '\nreturn wasm_bindgen;')(pointer => { highPointer ||= pointer >= 2 ** 31; });
  const wasm = binding.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 4096});
  const size = 768 * 1024 * 1024;
  for (let i = 0; i < 3; i++) wasm.__wbindgen_malloc(size, 1);
  assert.ok(wasm.memory.buffer.byteLength > 2 ** 31);
  const pointer = wasm.__wbindgen_malloc(65536, 1) >>> 0;
  assert.ok(pointer >= 2 ** 31);
  new Uint8Array(wasm.memory.buffer)[pointer] = 123;
  const input = fs.readFileSync('tests/fixtures/viewer-plan.json', 'utf8');
  const expected = binding.generate(input, '{"icon_mode":"service-name"}');
  const job = new binding.Generation(input, '{"icon_mode":"service-name","memory_budget_mib":4096}');
  let result;
  while (result === undefined) result = job.step(4096);
  assert.equal(result, expected);
  assert.ok(highPointer, 'wasm-bindgen must decode an output above the signed 2 GiB boundary');
  job.free();
  const between = new binding.Generation(input, '{"memory_budget_mib":4096,"icon_mode":"service-name"}');
  between.step(1);
  wasm.memory.grow(65536 - wasm.memory.buffer.byteLength / 65536);
  assert.equal(wasm.memory.buffer.byteLength, 4294967296);
  const view = new Uint8Array(wasm.memory.buffer);
  assert.equal(view.length, 4294967296);
  view[4294967295] = 17;
  assert.equal(view[4294967295], 17);
  assert.throws(() => wasm.memory.grow(1), RangeError);
  assert.throws(() => between.step(4096), error => JSON.parse(error).code === 'memory_limit');
  assert.throws(() => between.step(1), error => JSON.parse(error).code === 'generation_closed');
  between.free();
  assert.throws(() => new binding.Generation(input, '{"memory_budget_mib":4096}'), error => JSON.parse(error).code === 'memory_limit');
  console.log(`${directory}: allocator, string ABI above 2 GiB, 4 GiB buffer and safe high-water refusal passed`);
}
