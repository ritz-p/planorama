const assert = require('node:assert/strict');
const {By, Keys, wait} = require('./webdriver.cjs');

module.exports = function({driver, small, run, status, native}) {
  const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
  const rows = () => driver.execute_script('return inventory');
  const selected = () => driver.execute_script('return snapshotOptions().modules');
  const load = json => driver.execute_script(`
    const transfer = new DataTransfer();
    transfer.items.add(new File([arguments[0]], 'plan.json'));
    element('file').files = transfer.files;
    element('file').dispatchEvent(new Event('change', {bubbles: true}));
  `, json);
  const loaded = () => wait(() => find('#inventory-status').text.startsWith('Modules from'), 'module inventory');
  const search = query => driver.execute_script(`
    element('module-search').value = arguments[0];
    element('module-search').dispatchEvent(new Event('input', {bubbles: true}));
  `, query);
  const selector = `module.app[${JSON.stringify(' a,b:c.d"e\\f ')}]`;
  const child = `${selector}.module.child[0]`;
  const resource = {address: `${selector}.terraform_data.item`, type: 'terraform_data', change: {actions: ['create'], after: {}}};
  const json = JSON.stringify({format_version: '1.2', resource_changes: [
    {address: 'terraform_data.root', type: 'terraform_data'}, resource, resource,
    {...resource, deposed: 'old'},
    {address: `${child}.data.aws_region.current`, type: 'aws_region', mode: 'data'},
  ], planned_values: {root_module: {child_modules: [{address: 'module.empty[0]'}]}},
  configuration: {root_module: {module_calls: {app: {module: {}}, unknown: {module: {}}}}}});
  const result = run(json, {}, 'inspect');
  assert.equal(result.ok, true);
  const lines = result.modules.map(row => `${row.selector ?? '(root; not selectable)'}\t${row.instance ? 'instance' : 'configuration (aggregate)'}\t${row.direct}\t${row.subtree}`).join('\n');
  assert.ok(native(json, ['--list-modules']).endsWith(lines + '\n'), 'Native and Wasm use identical original inventory');
  assert.deepEqual(result.modules.find(row => row.selector === selector), {selector, instance: true, direct: 2, subtree: 3});
  for (const [input, code] of [['secret{', 'invalid_json'], ['{"format_version":"1.0","values":{}}', 'unsupported_input'], [' '.repeat(32 * 1024 * 1024 + 1), 'input_too_large']]) {
    assert.equal(run(input, {}, 'inspect').error.code, code);
  }
  load(json);
  loaded();
  assert.deepEqual(rows(), result.modules);
  assert.equal(find('#module-inventory input').is_enabled(), false, 'Root has no selector');
  search(selector);
  find('#module-inventory input').send_keys(Keys.SPACE);
  assert.deepEqual(selected(), [selector]);
  assert.equal(driver.execute_script('return document.activeElement.value'), selector);
  search('unknown');
  find('#module-inventory input').send_keys(Keys.SPACE);
  assert.deepEqual(selected(), [selector, 'module.unknown']);
  assert.ok(find('#module-count').text.startsWith('2 selectors selected.'));
  search(selector);
  assert.equal(driver.execute_script("return element('module-inventory').querySelector('input').checked"), true);
  find('#module-inventory input').send_keys(Keys.SPACE);
  assert.deepEqual(selected(), ['module.unknown']);
  find('#reset-settings').click();
  assert.deepEqual(selected(), []);
  assert.deepEqual(rows(), result.modules, 'Reset retains completed inventory');
  search(selector);
  find('#module-inventory input').click();
  driver.execute_script(`
    window.moduleBlobs = new Map();
    window.moduleCreateUrl = URL.createObjectURL;
    URL.createObjectURL = blob => {
      const url = moduleCreateUrl(blob);
      moduleBlobs.set(url, blob);
      return url;
    };
  `);
  driver.execute_script("element('generate-form').requestSubmit()");
  wait(() => status().startsWith('Generated in'), 'generation with picked selector');
  driver.execute_script(`
    window.pickedContent = null;
    moduleBlobs.get(element('download').href).text().then(content => { window.pickedContent = content; });
    URL.createObjectURL = moduleCreateUrl;
  `);
  assert.equal(wait(() => driver.execute_script('return window.pickedContent'), 'picked output'), native(json, ['--modules', selector]));
  const many = JSON.stringify({format_version: '1.2', resource_changes: [], planned_values: {root_module: {
    child_modules: Array.from({length: 120}, (_, i) => ({address: `module.m${String(i).padStart(3, '0')}`})),
  }}});
  load(many);
  loaded();
  assert.deepEqual(selected(), [], 'Replacement clears old selections');
  assert.equal(driver.execute_script("return element('module-inventory').children.length"), 50);
  find('#module-inventory label:nth-child(2) input').send_keys(Keys.SPACE);
  find('#module-next').send_keys(Keys.ENTER);
  find('#module-inventory input').send_keys(Keys.SPACE);
  assert.equal(selected().length, 2);
  search('m119');
  find('#module-inventory input').send_keys(Keys.SPACE);
  search('missing');
  assert.ok(find('#module-count').text.startsWith('3 selectors selected. 0 of 121 rows match.'));
  assert.equal(selected().length, 3, 'All selections survive search and pagination');
  driver.execute_script(`
    window.savedFileText = File.prototype.text;
    window.pendingReads = [];
    File.prototype.text = function() {
      const file = this;
      return new Promise(resolve => pendingReads.push(() => resolve(savedFileText.call(file))));
    };
  `);
  load(json);
  load(small);
  driver.execute_script('pendingReads[1]();');
  loaded();
  const current = rows();
  driver.execute_script('pendingReads[0]();');
  assert.deepEqual(rows(), current, 'Older read cannot replace newer inventory');
  for (const action of ['cancel', 'reset-settings']) {
    load(json);
    find('#' + action).click();
    driver.execute_script('pendingReads.at(-1)();');
    assert.deepEqual(rows(), []);
    assert.equal(driver.execute_script('return worker === undefined'), true);
  }
  driver.execute_script('File.prototype.text = savedFileText;');
  driver.execute_script(`
    window.InventoryWorker = Worker;
    window.Worker = class extends InventoryWorker {
      postMessage(data) { window.oldInventoryCompletion = this.onmessage; }
    };
  `);
  load(json);
  wait(() => driver.execute_script('return !!worker'), 'pending inventory worker');
  find('#cancel').click();
  driver.execute_script(`
    oldInventoryCompletion({data: {ok: true, modules: [{selector: 'STALE'}]}});
    window.Worker = InventoryWorker;
  `);
  assert.deepEqual(rows(), []);
  load('secret{');
  wait(() => find('#inventory-status').text === 'Input is not valid plan JSON.', 'safe inventory error');
  load(small);
  loaded();
  find('#reset-settings').click();
};
