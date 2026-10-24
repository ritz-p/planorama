const assert = require('node:assert/strict');
const {By, Keys, wait} = require('./webdriver.cjs');

module.exports = function({driver, run, native}) {
  const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
  const load = json => driver.execute_script(`
    const transfer = new DataTransfer();
    transfer.items.add(new File([arguments[0]], 'resources.json'));
    element('file').files = transfer.files;
    element('file').dispatchEvent(new Event('change', {bubbles: true}));
  `, json);
  const loaded = () => wait(() => find('#inventory-status').text.startsWith('Modules from'), 'resource inventory');
  const search = value => driver.execute_script(`element('resource-search').value = arguments[0]; element('resource-search').dispatchEvent(new Event('input', {bubbles: true}));`, value);
  const focus = () => driver.execute_script('return snapshotOptions().focus ?? null');
  const module = `module.app[${JSON.stringify(' a,b:c.d"e\\f ')}].module.child[0]`;
  const address = `${module}.terraform_data.item[${JSON.stringify(' x,"\\ ')}]`;
  const item = {address, type: 'terraform_data', change: {actions: ['create'], after: {secret: 'NEVER_EXPOSE'}}};
  const input = {format_version: '1.2', resource_changes: [item,
    {address: 'data.aws_region.current', type: 'aws_region', mode: 'data'},
    {address: 'terraform_data.history', type: 'terraform_data'},
    {address: 'terraform_data.history', type: 'terraform_data', deposed: 'old'},
    ...Array.from({length: 110}, (_, i) => ({address: `terraform_data.r${String(i).padStart(3, '0')}`, type: 'terraform_data'})),
  ]};
  const json = JSON.stringify(input);
  const result = run(json, {}, 'inspect');
  assert.equal(result.ok, true);
  assert.equal(result.resources.length, 113);
  assert.deepEqual(driver.execute_script('return progressStages'), [], 'Inspection never enters layout or routing');
  assert.ok(!JSON.stringify(result.resources).includes('NEVER_EXPOSE'));
  assert.deepEqual(result.resources.find(row => row.address === address), {
    state: null, selector: address, address, resource_type: 'terraform_data', module, mode: 'managed', current: 1, deposed: 0,
  });
  assert.equal(run(json, {focus: 'terraform_data.history'}).error.code, 'selection_failed');
  load(json);
  loaded();
  assert.equal(driver.execute_script("return element('resource-inventory').children.length"), 50);
  search(address);
  find('#resource-inventory input').send_keys(Keys.SPACE);
  assert.equal(focus(), address);
  assert.equal(driver.execute_script('return document.activeElement.value'), address);
  const pickedOptions = driver.execute_script('return snapshotOptions()');
  const picked = run(json, pickedOptions);
  assert.equal(picked.ok, true);
  find('#clear-focus').click();
  assert.equal(focus(), null);
  driver.execute_script("element('focus').value = arguments[0]; element('focus').dispatchEvent(new Event('input', {bubbles: true}));", address);
  assert.equal(driver.execute_script("return element('resource-inventory').querySelector('input').checked"), true);
  assert.equal(run(json, driver.execute_script('return snapshotOptions()')).content, picked.content);
  assert.equal(picked.content, native(json, ['--focus', address]));
  for (const options of [
    {focus: address, focus_depth: 0, modules: [module], changed_only: true},
    {focus: 'data.aws_region.current', focus_depth: 0},
  ]) {
    const args = ['--focus', options.focus, '--focus-depth', '0'];
    if (options.modules) args.push('--modules', module, '--changed-only');
    const result = run(json, options);
    if (options.focus === 'data.aws_region.current') {
      assert.equal(result.error.code, 'selection_failed', 'Input presence does not imply a rendered focus target');
      assert.throws(() => native(json, args));
    } else {
      assert.equal(result.ok, true, JSON.stringify(result));
      assert.equal(result.content, native(json, args));
    }
  }
  search('');
  find('#resource-next').send_keys(Keys.ENTER);
  assert.ok(find('#resource-count').text.endsWith('Showing 51–100.'));
  search('missing');
  assert.equal(focus(), address);
  assert.ok(find('#resource-count').text.startsWith('0 of 113'));
  search('aws_region');
  assert.ok(find('#resource-inventory').text.includes('data; state: single plan (unnamed)'));
  find('#reset-settings').click();
  assert.equal(focus(), null);
  assert.equal(driver.execute_script('return resources.length'), 113);
  driver.execute_script(`window.savedResourceText = File.prototype.text; window.resourceReads = [];
    File.prototype.text = function() { const file = this; return new Promise(resolve => resourceReads.push(() => resolve(savedResourceText.call(file)))); };`);
  load(json);
  load(JSON.stringify({format_version: '1.2', resource_changes: []}));
  driver.execute_script('resourceReads[1]();');
  loaded();
  driver.execute_script('resourceReads[0]();');
  assert.equal(driver.execute_script('return resources.length'), 0);
  for (const action of ['cancel', 'reset-settings', 'clear-focus']) {
    load(json);
    find('#' + action).click();
    driver.execute_script('resourceReads.at(-1)();');
    assert.equal(driver.execute_script('return resources.length'), 0);
  }
  driver.execute_script('File.prototype.text = savedResourceText;');
  load('secret{');
  wait(() => find('#inventory-status').text === 'Input is not valid plan JSON.', 'resource error');
  assert.equal(driver.execute_script('return resources.length'), 0);
  load(json);
  loaded();
  find('#inspect-resources').click();
  loaded();
  assert.equal(driver.execute_script('return resources.length'), 113);
  search(address);
  find('#resource-inventory input').click();
  load(JSON.stringify({format_version: '1.2', resource_changes: []}));
  assert.equal(focus(), null, 'File replacement clears focus');
  loaded();
  find('#reset-settings').click();
};
