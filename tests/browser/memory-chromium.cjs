const assert = require('node:assert/strict');
const {wait} = require('./webdriver.cjs');

module.exports = (driver, source, wasmUrl, input) => {
  driver.execute_script(`
    window.memoryTest = null;
    const source = arguments[0], url = arguments[1], input = arguments[2];
    (async () => {
      const module = await WebAssembly.compile(await (await fetch(url)).arrayBuffer());
      const results = [];
      for (const named of [false, true]) for (const mib of [512, 1024, 2048, 4096, 512]) {
        const body = source + '\\nself.onmessage = ({data}) => {\\n' +
          'try { const wasm = wasm_bindgen.initSync({module:data.module,memory_budget_mib:data.mib});' +
          'const options = JSON.stringify({memory_budget_mib:data.mib,icon_mode:"service-name"});' +
          'const job = data.named ? wasm_bindgen.Generation.named(JSON.stringify([{state:"a",json:data.input},{state:"b",json:data.input}]),options) : new wasm_bindgen.Generation(data.input,options);' +
          'const stages = [];' +
          'const step = () => { try { const result=job.step_with_progress(64, stage => stages.push(stage)); if(result===undefined) setTimeout(step,0);' +
          'else { job.free(); self.postMessage({result,named:data.named,stages,capacity:wasm.memory.buffer.byteLength}); }' +
          '} catch(error) { self.postMessage({error:String(error)}); } }; step();' +
          '} catch(error) { self.postMessage({error:String(error)}); } };';
        const blob = URL.createObjectURL(new Blob([body], {type:'text/javascript'}));
        const worker = new Worker(blob);
        try {
          results.push(await new Promise((resolve, reject) => {
            worker.onmessage = event => resolve(event.data);
            worker.onerror = () => reject(new Error('Memory Worker failed'));
            worker.postMessage({module, input, mib, named});
          }));
        } finally { worker.terminate(); URL.revokeObjectURL(blob); }
      }
      window.memoryTest = {results};
    })().catch(error => { window.memoryTest = {error:String(error)}; });
  `, source, wasmUrl, input);
  const result = wait(() => driver.execute_script('return window.memoryTest;'), 'Chromium memory budget workers', 30000);
  assert.equal(result.error, undefined);
  for (const entry of result.results) {
    assert.equal(entry.error, undefined);
    assert.equal(entry.result, result.results.find(value => value.named === entry.named).result);
    assert.equal(entry.stages[0], 'parsing');
    assert.equal(entry.stages.at(-1), 'output');
    assert.ok(entry.stages.includes('routing'));
    assert.ok(entry.capacity < 512 * 1024 * 1024);
  }
};
