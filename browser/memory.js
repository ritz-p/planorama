((raw, capabilities) => {
  const allowed = Object.freeze(capabilities.allowed_mib);
  Object.freeze(capabilities);
  let selected, exports, pending, failed = false;
  const failure = (code, message) => { throw JSON.stringify({code, message}); };
  const invalid = () => failure('invalid_options', 'Unsupported or invalid generation options.');
  const mismatch = () => failure('memory_budget_mismatch', 'Use a new Worker to change the memory budget.');
  const choose = options => {
    if ('memory' in options) invalid();
    const mib = options.memory_budget_mib === undefined ? capabilities.default_mib : options.memory_budget_mib;
    if (!Number.isInteger(mib) || !allowed.includes(mib)) invalid();
    if (selected !== undefined && selected !== mib) mismatch();
    if (failed) failure('memory_unavailable', 'Discard this Worker and retry.');
    selected = mib;
    return mib;
  };
  const memory = mib => {
    try {
      return new WebAssembly.Memory({initial: capabilities.initial_pages, maximum: mib * 16});
    } catch {
      failed = true;
      failure('memory_unavailable', 'This runtime could not create the requested memory. Use a new Worker to retry.');
    }
  };
  const finish = result => {
    raw.configure_memory_budget(selected);
    exports = result;
    return result;
  };
  const normalize = (value, key) => value && Object.getPrototypeOf(value) === Object.prototype ? value : {[key]: value};
  const initSync = value => {
    const options = normalize(value, 'module');
    const mib = choose(options);
    if (exports) return exports;
    if (pending) mismatch();
    try {
      return finish(raw.initSync({...options, memory: memory(mib)}));
    } catch (error) {
      failed = true;
      throw error;
    }
  };
  const init = value => {
    const options = normalize(value, 'module_or_path');
    const mib = choose(options);
    if (exports) return Promise.resolve(exports);
    if (pending) return pending;
    pending = (async () => {
      try {
        return finish(await raw({...options, memory: memory(mib)}));
      } catch (error) {
        failed = true;
        throw error;
      }
    })();
    return pending;
  };
  wasm_bindgen = Object.assign(init, raw, {initSync, memory_capabilities: () => JSON.stringify(capabilities)});
  delete wasm_bindgen.configure_memory_budget;
})(wasm_bindgen, MEMORY_CAPABILITIES);
