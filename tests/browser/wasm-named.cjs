const assert = require('node:assert/strict');
const {readFileSync, writeFileSync, mkdtempSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const {join} = require('node:path');
const {execFileSync} = require('node:child_process');
const {wait} = require('./webdriver.cjs');

module.exports = function({driver, root, binary}) {
  const path = join(root, 'tests/fixtures/browser-named.json');
  const json = readFileSync(path, 'utf8');
  const inputs = ['network', 'application'].map(state => ({state, json}));
  const run = (value = inputs, options = {}, operation = 'generate') => {
    driver.execute_script(`
      window.namedResult = null;
      const w = new Worker(assets.url);
      w.onmessage = ({data}) => { if (data.type === 'progress') return; window.namedResult = data; w.terminate(); };
      w.onerror = () => { window.namedResult = {ok: false, trap: true}; w.terminate(); };
      w.postMessage({module: assets.module, inputs: arguments[0], options: arguments[1], operation: arguments[2]});
    `, value, options, operation);
    return wait(() => driver.execute_script('return window.namedResult ? [window.namedResult] : null'), 'named result', 60000)[0];
  };
  const inventory = run(inputs, {}, 'inspect');
  const mappingDirectory = mkdtempSync(join(tmpdir(), 'named-mappings-'));
  try {
    const mappedInputs = ['producer', 'consumer'].map(state => ({state, json:readFileSync(join(root, `tests/fixtures/multi/${state}.json`), 'utf8')}));
    mappedInputs.push({state:'other', json:mappedInputs[0].json});
    for (const special of [false, true]) {
      const remote = special ? 'data.terraform_remote_state.network["</script><svg/onload=alert(1)>&=\\\""]' : 'data.terraform_remote_state.network';
      const current = mappedInputs.map(input => ({...input, json: input.state === 'consumer' ? input.json.replaceAll('data.terraform_remote_state.network', JSON.stringify(remote).slice(1,-1)) : input.json}));
      for (const input of current) writeFileSync(join(mappingDirectory, input.state + '.json'), input.json);
      const remote_state_mappings = [{consumer:'consumer', remote, producer:'producer'}];
      for (const [options, args] of [[{}, []], [{format:'html'}, ['--format','html']], [{focus:'consumer:aws_instance.app', focus_depth:0}, ['--focus','consumer:aws_instance.app','--focus-depth','0']], [{focus:'consumer:aws_instance.app'}, ['--focus','consumer:aws_instance.app']], [{changed_only:true}, ['--changed-only']]]) {
        const result = run(current, {...options, remote_state_mappings});
        assert.equal(result.ok, true, JSON.stringify(result));
        const native = execFileSync(binary, [...current.flatMap(input => ['--state', input.state + '=' + join(mappingDirectory, input.state + '.json')]), '--remote-state', `consumer:${remote}=producer`, '-o', '-', ...args], {encoding:'utf8', maxBuffer:32*1024*1024});
        assert.equal(result.content, native);
        assert.ok(!result.content.includes('data-source-state="other"'));
        assert.ok(!result.content.includes('</script><svg/onload'));
      }
    }
    const mapping = {consumer:'consumer', remote:'data.terraform_remote_state.network', producer:'producer'};
    for (const [remote_state_mappings, code] of [
      [[{...mapping, producer:'missing'}], 'unknown_remote_state_mapping'],
      [[{...mapping, remote:'SECRET\ninvalid'}], 'invalid_remote_state_mapping'],
      [[{...mapping, remote:'data.terraform_remote_state.missing'}], 'invalid_remote_state_mapping'],
      [[mapping, {...mapping, producer:'other'}], 'ambiguous_remote_state_mapping'],
      [[{...mapping, extra:'SECRET'}], 'invalid_options'],
      [Array(257).fill(mapping), 'resource_limit'],
      [[{...mapping, remote:'x'.repeat(65537)}], 'resource_limit'],
    ]) {
      const result = run(mappedInputs, {remote_state_mappings});
      assert.equal(result.error.code, code);
      assert.ok(!JSON.stringify(result).includes('SECRET'));
    }
    const incomplete = mappedInputs.map(input => ({...input, json:input.state === 'producer' ? JSON.stringify({...JSON.parse(input.json), configuration:{root_module:{outputs:{both:{expression:{constant_value:'SECRET'}}}}}}) : input.json}));
    const result = run(incomplete, {remote_state_mappings:[mapping]});
    assert.equal(result.ok, true);
    assert.ok(result.diagnostics.includes('producer output is missing'));
    assert.ok(result.diagnostics.includes('producer output provenance is unresolved'));
    assert.ok(!JSON.stringify(result).includes('SECRET'));
  } finally { rmSync(mappingDirectory, {recursive:true, force:true}); }
  assert.equal(inventory.ok, true);
  assert.equal(inventory.resources.length, 6);
  for (const state of ['network', 'application']) {
    assert.equal(inventory.modules.find(row => row.state === state && row.selector === null).subtree, 4);
    const row = inventory.resources.find(row => row.selector === `${state}:module.app.terraform_data.history`);
    assert.deepEqual([row.current, row.deposed], [1, 1]);
  }
  for (const [options, args] of [
    [{}, []],
    [{format: 'html', html_diagnostics: true, html_semantic_diagnostics: true}, ['--format', 'html', '--html-diagnostics', '--html-semantic-diagnostics']],
    [{icon_mode: 'service-name', modules: ['network:module.app']}, ['--icon-mode', 'service-name', '--modules', 'network:module.app']],
    [{focus: 'application:module.app.terraform_data.item', focus_depth: 0}, ['--focus', 'application:module.app.terraform_data.item', '--focus-depth', '0']],
  ]) {
    const result = run(inputs, options);
    assert.equal(result.ok, true, JSON.stringify(result));
    assert.equal(result.content, execFileSync(binary, ['--state', `network=${path}`, '--state', `application=${path}`, '-o', '-', ...args], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024}));
  }
  for (const [value, code] of [
    [[], 'empty_inputs'], [[{state: '', json}], 'invalid_state_name'],
    [[{state: 'bad:name', json}], 'invalid_state_name'],
    [[inputs[0], inputs[0]], 'duplicate_state_name'],
    [[inputs[0], {state: 'other', json: 'SECRET broken'}], 'invalid_json'],
  ]) {
    const result = run(value, {}, 'inspect');
    assert.equal(result.error.code, code);
    assert.ok(!JSON.stringify(result).includes('SECRET'));
  }
  assert.equal(run(inputs, {focus: 'network:module.app.terraform_data.history'}).error.code, 'selection_failed');
  const setup = () => driver.execute_script(`
    element('named-mode').checked = true; element('named-mode').onchange();
    element('named-inputs').replaceChildren();
    for (const input of arguments[0]) {
      element('add-input').click();
      const row = element('named-inputs').lastElementChild;
      row.querySelector('.state-name').value = input.state;
      const transfer = new DataTransfer(); transfer.items.add(new File([input.json], 'same-name.json'));
      row.querySelector('input[type=file]').files = transfer.files;
    }
    inputsChanged();
  `, inputs);
  const status = () => driver.execute_script("return element('status').textContent");
  setup();
  driver.execute_script('inspectModules()');
  wait(() => driver.execute_script('return resources.length === 6'), 'named inventory UI');
  driver.execute_script("element('module-inventory').querySelector('input[value=\"network:module.app\"]').click()");
  assert.deepEqual(driver.execute_script('return snapshotOptions().modules'), ['network:module.app']);
  driver.execute_script("element('resource-inventory').querySelector('input[value=\"application:module.app.terraform_data.item\"]').click()");
  assert.equal(driver.execute_script('return snapshotOptions().focus'), 'application:module.app.terraform_data.item');
  driver.execute_script("element('reset-settings').click(); element('generate-form').requestSubmit()");
  wait(() => status().startsWith('Generated'), 'named generation UI');
  assert.equal(driver.execute_script("return element('preview').getAttribute('sandbox')"), 'allow-scripts');
  driver.execute_script("element('named-inputs').firstElementChild.querySelector('.state-name').value = 'renamed'; element('named-inputs').firstElementChild.querySelector('.state-name').dispatchEvent(new Event('input', {bubbles: true}))");
  assert.deepEqual(driver.execute_script('return [inventory.length, resources.length, snapshotOptions().modules.length, element("download").hidden]'), [0, 0, 0, true]);
  driver.execute_script("element('named-inputs').lastElementChild.querySelector('.state-name').value = 'renamed'; element('generate-form').requestSubmit()");
  assert.ok(status().includes('duplicate state name'));
  setup();
  driver.execute_script(`
    window.originalText = File.prototype.text;
    window.readCount = 0;
    File.prototype.text = function() { return ++window.readCount === 2 ? Promise.reject(new Error('SECRET')) : window.originalText.call(this); };
    element('generate-form').requestSubmit();
  `);
  wait(() => status().includes('Input 2: could not read'), 'safe partial read failure');
  assert.ok(!status().includes('SECRET'));
  driver.execute_script('File.prototype.text = window.originalText');
  driver.execute_script(`
    window.resolveOldRead = null;
    File.prototype.text = function() { return new Promise(resolve => { window.resolveOldRead = resolve; }); };
    element('generate-form').requestSubmit();
  `);
  wait(() => driver.execute_script('return !!window.resolveOldRead'), 'pending read');
  driver.execute_script(`
    element('named-inputs').lastElementChild.querySelector('button').click();
    window.resolveOldRead(arguments[0]); File.prototype.text = window.originalText;
  `, json);
  assert.equal(driver.execute_script('return worker === undefined'), true);
  driver.execute_script("element('generate-form').requestSubmit()");
  wait(() => status().startsWith('Generated'), 'retry after removal');
  for (const edit of ['add', 'replace', 'rename', 'settings', 'cancel']) {
    setup();
    driver.execute_script(`
      window.realWorker = Worker;
      window.capturedWorker = null;
      window.Worker = class {
        postMessage(data) { window.capturedWorker = this; this.snapshot = data; }
        terminate() { this.terminated = true; }
      };
      element('generate-form').requestSubmit();
    `);
    wait(() => driver.execute_script('return !!window.capturedWorker'), 'pending named Worker');
    assert.deepEqual(driver.execute_script('return capturedWorker.snapshot.inputs.map(input => input.state)'), ['network', 'application']);
    driver.execute_script(`
      const edit = arguments[0];
      if (edit === 'add') element('add-input').click();
      if (edit === 'replace') element('named-inputs').firstElementChild.querySelector('input[type=file]').dispatchEvent(new Event('change', {bubbles: true}));
      if (edit === 'rename') {
        const name = element('named-inputs').firstElementChild.querySelector('.state-name');
        name.value = 'new'; name.dispatchEvent(new Event('input', {bubbles: true}));
      }
      if (edit === 'settings') element('changed').dispatchEvent(new Event('input', {bubbles: true}));
      if (edit === 'cancel') element('cancel').click();
      capturedWorker.onmessage({data: {ok: true, content: 'STALE', diagnostics: [], elapsed_ms: 0}});
      window.Worker = window.realWorker;
    `, edit);
    assert.deepEqual(driver.execute_script('return [capturedWorker.terminated, worker === undefined, element("download").hidden]'), [true, true, true], edit);
    assert.ok(!status().startsWith('Generated'), edit);
  }
  setup();
  driver.execute_script(`
    for (const row of element('named-inputs').children) {
      const transfer = new DataTransfer(); transfer.items.add(new File([new Uint8Array(17 * 1024 * 1024)], 'large.json'));
      row.querySelector('input[type=file]').files = transfer.files;
    }
    element('generate-form').requestSubmit();
  `);
  assert.ok(status().includes('32 MiB'));
  driver.execute_script("element('named-mode').checked = false; element('named-mode').onchange(); element('reset-settings').click()");
  console.log('Named inputs: native/Wasm parity, identity, selection, validation, read failure, cancellation, retry and aggregate size passed');
};
