const assert = require('node:assert/strict');
const {By, wait} = require('./webdriver.cjs');

module.exports = ({driver, small, submit, status}) => {
  const options = () => driver.execute_script('return snapshotOptions()');
  const click = id => driver.find_element(By.CSS_SELECTOR, '#' + id).click();
  const load = (text, name = 'settings.json') => driver.execute_script(`
    const transfer = new DataTransfer();
    transfer.items.add(new File([arguments[0]], arguments[1], {type: 'application/json'}));
    document.getElementById('settings-file').files = transfer.files;
    document.getElementById('settings-error').textContent = '';
    document.getElementById('settings-file').dispatchEvent(new Event('change', {bubbles: true}));
  `, text, name);
  const error = () => driver.find_element(By.CSS_SELECTOR, '#settings-error').text;
  click('reset-settings');
  const defaults = options();
  const selected = {...defaults, format: 'html', changed_only: true, address_format: 'terraform',
    hide_relationship_legend: true, omit_relationship_legend: true,
    html_diagnostics: true, html_semantic_diagnostics: true,
    focus: 'state:module.app["a b,c.d\\"e"].terraform_data.x', focus_depth: 0,
    modules: ['state:module.app["a b,c.d\\"e"]', 'module.not_in_inventory'],
    spatial_types: ['aws_ssm_parameter', 'aws_iam_role']};
  const file = settings => JSON.stringify({schema_version: 1, settings});
  try {
    const planCount = driver.execute_script('return document.getElementById("file").files.length');
    load(file(selected));
    wait(() => status().startsWith('Settings imported.'), 'settings import');
    assert.deepEqual(options(), selected);
    assert.equal(driver.execute_script('return document.activeElement.id'), 'import-settings');
    assert.equal(driver.execute_script('return document.getElementById("file").files.length'), planCount);
    assert.equal(driver.find_elements(By.CSS_SELECTOR, '#modules input').length, 2);
    driver.execute_script(`
      window.savedSettingsDownload = null;
      window.settingsCreateURL = URL.createObjectURL;
      window.settingsRevokeURL = URL.revokeObjectURL;
      window.settingsAnchorClick = HTMLAnchorElement.prototype.click;
      window.settingsUrls = new Map();
      URL.createObjectURL = blob => { const url = settingsCreateURL(blob); settingsUrls.set(url, blob); return url; };
      URL.revokeObjectURL = url => { settingsUrls.delete(url); settingsRevokeURL(url); };
      HTMLAnchorElement.prototype.click = function() {
        if (this.download !== 'planorama-settings.json') return settingsAnchorClick.call(this);
        settingsUrls.get(this.href).text().then(text => { window.savedSettingsDownload = text; });
      };
    `);
    click('export-settings');
    const exported = wait(() => driver.execute_script('return savedSettingsDownload'), 'settings export');
    assert.deepEqual(JSON.parse(exported), {schema_version: 1, settings: selected});
    wait(() => driver.execute_script('return settingsUrls.size === 0'), 'settings URL revoked');
    click('reset-settings');
    load(exported);
    wait(() => status().startsWith('Settings imported.'), 'round trip import');
    assert.deepEqual(options(), selected);
    for (const text of ['{', file({format: 'svg', html_diagnostics: true}),
      '{"schema_version":999,"settings":{}}', file({modules: ['']}), file({plan: 'PRIVATE'}),
      ' '.repeat(65537)]) {
      load(text);
      wait(() => error().startsWith('Could not import'), 'invalid settings rejected');
      assert.deepEqual(options(), selected);
      assert.ok(!error().includes('PRIVATE'));
    }
    driver.execute_script(`
      window.settingsOriginalText = File.prototype.text;
      File.prototype.text = function() {
        if (this.name === 'unreadable.json') return Promise.reject(new Error('PRIVATE'));
        return settingsOriginalText.call(this);
      };
    `);
    load(file(defaults), 'unreadable.json');
    wait(() => error().startsWith('Could not import'), 'unreadable settings rejected');
    assert.deepEqual(options(), selected);
    click('reset-settings');
    driver.execute_script(`
      window.settingsOriginalWorker = Worker;
      window.settingsTerminations = 0;
      window.Worker = class extends settingsOriginalWorker {
        postMessage(data) {
          if (data.operation === 'inspect') return super.postMessage(data);
          window.settingsStaleHandler = this.onmessage;
        }
        terminate() { settingsTerminations++; return super.terminate(); }
      };
    `);
    submit(small, 'svg');
    wait(() => driver.execute_script('return !!window.settingsStaleHandler'), 'active generation');
    const request = driver.execute_script('return serial');
    load(file({changed_only: 'invalid'}));
    wait(() => error().startsWith('Could not import'), 'invalid import during generation');
    assert.equal(driver.execute_script('return serial'), request);
    assert.equal(driver.execute_script('return !!worker'), true);
    load(file(defaults));
    wait(() => status().startsWith('Settings imported.'), 'import cancels generation');
    assert.equal(driver.execute_script('return worker === undefined && settingsTerminations > 0'), true);
    driver.execute_script('settingsStaleHandler({data:{ok:true,content:"STALE",diagnostics:[]}});');
    assert.equal(driver.execute_script('return document.getElementById("download").hasAttribute("href")'), false);
    assert.ok(status().startsWith('Settings imported.'));
    driver.execute_script(`
      window.Worker = settingsOriginalWorker;
      File.prototype.text = function() {
        if (this.name !== 'plan.json') return settingsOriginalText.call(this);
        return new Promise(resolve => { window.settingsFinishPlanRead = () => resolve(settingsOriginalText.call(this)); });
      };
    `);
    submit(small, 'svg');
    load(file(selected));
    wait(() => status().startsWith('Settings imported.'), 'import cancels plan read');
    driver.execute_script('settingsFinishPlanRead();');
    assert.equal(driver.execute_script('return worker === undefined'), true);
    assert.deepEqual(options(), selected);
    driver.execute_script(`
      File.prototype.text = function() {
        return new Promise(resolve => { window.settingsFinishImport = () => resolve(settingsOriginalText.call(this)); });
      };
    `);
    load(file(defaults));
    click('reset-settings');
    driver.execute_script('settingsFinishImport();');
    assert.ok(status().startsWith('Settings reset'));
    assert.deepEqual(options(), defaults);
  } finally {
    driver.execute_script(`
      if (window.settingsOriginalText) File.prototype.text = settingsOriginalText;
      if (window.settingsOriginalWorker) window.Worker = settingsOriginalWorker;
      if (window.settingsCreateURL) URL.createObjectURL = settingsCreateURL;
      if (window.settingsRevokeURL) URL.revokeObjectURL = settingsRevokeURL;
      if (window.settingsAnchorClick) HTMLAnchorElement.prototype.click = settingsAnchorClick;
    `);
    click('reset-settings');
  }
};
