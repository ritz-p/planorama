self.onmessage = ({data}) => {
  try {
    const wasm = wasm_bindgen.initSync({module: data.module});
    const started = performance.now();
    const result = JSON.parse(wasm_bindgen.generate(data.json, JSON.stringify(data.options)));
    self.postMessage({ok: true, ...result, elapsed_ms: performance.now() - started, memory_bytes: wasm.memory.buffer.byteLength});
  } catch (error) {
    let failure = {code: 'generation_failed', message: 'Diagram generation failed.'};
    try {
      const parsed = JSON.parse(error);
      if (['input_too_large', 'invalid_json', 'unsupported_input', 'invalid_options', 'selection_failed', 'generation_failed'].includes(parsed.code)) failure = parsed;
    } catch {
      // Traps and runtime exceptions must not expose input or implementation details.
    }
    self.postMessage({ok: false, error: failure});
  }
};
