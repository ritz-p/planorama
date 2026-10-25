self.onmessage = ({data}) => {
  try {
    const wasm = wasm_bindgen.initSync({module: data.module});
    const started = performance.now();
    let lastStage;
    const progress = stage => {
      if (stage === lastStage) return;
      lastStage = stage;
      self.postMessage({type: 'progress', request: data.request, stage});
    };
    const result = data.inputs ? (data.operation === 'inspect'
      ? JSON.parse(wasm_bindgen.inspect_named_inputs(JSON.stringify(data.inputs)))
      : JSON.parse(wasm_bindgen.generate_named_with_progress(JSON.stringify(data.inputs), JSON.stringify(data.options), progress))) : data.operation === 'inspect'
      ? JSON.parse(wasm_bindgen.inspect_input(data.json))
      : JSON.parse(wasm_bindgen.generate_with_progress(data.json, JSON.stringify(data.options), progress));
    self.postMessage({ok: true, ...result, elapsed_ms: performance.now() - started, memory_bytes: wasm.memory.buffer.byteLength});
  } catch (error) {
    let failure = {code: 'generation_failed', message: 'Diagram generation failed.'};
    try {
      const parsed = JSON.parse(error);
      if (['invalid_state_name', 'duplicate_state_name', 'empty_inputs', 'input_too_large', 'invalid_json', 'unsupported_input', 'invalid_options', 'selection_failed', 'generation_failed', 'resource_limit', 'invalid_remote_state_mapping', 'unknown_remote_state_mapping', 'ambiguous_remote_state_mapping'].includes(parsed.code)) failure = parsed;
    } catch {
      // Traps and runtime exceptions must not expose input or implementation details.
    }
    self.postMessage({ok: false, error: failure});
  }
};
