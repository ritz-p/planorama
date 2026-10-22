const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {join} = require('node:path');
const {By, Keys, wait} = require('./webdriver.cjs');

module.exports = function checkOptions({driver, small, run, submit, status, native}) {
  const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
  const set = (id, value) => driver.execute_script(`
    const input = document.getElementById(arguments[0]);
    if (input.type === 'checkbox') input.checked = arguments[1];
    else input.value = arguments[1];
    input.dispatchEvent(new Event('input', {bubbles: true}));
  `, id, value);
  const reset = () => find('#reset-settings').click();
  const options = () => driver.execute_script('return snapshotOptions()');
  const defaults = {format: 'svg', changed_only: false, address_format: 'qualified', icon_mode: 'icon', hide_relationship_legend: false, modules: [], spatial_types: []};
  assert.deepEqual(options(), defaults);
  assert.equal(find('#focus-depth').is_enabled(), false);
  assert.equal(find('#settings-error').get_attribute('role'), 'alert');
  assert.equal(find('#status').get_attribute('role'), 'status');
  for (const id of ['format', 'changed', 'address-format', 'icon-mode', 'hide-legend', 'focus', 'focus-depth']) {
    assert.ok(find('#' + id).accessible_name, `${id} has a label`);
  }

  driver.execute_script(`
    window.uiRequests = [];
    window.uiBlobs = new Map();
    window.originalCreateUrl = URL.createObjectURL;
    window.originalRevokeUrl = URL.revokeObjectURL;
    URL.createObjectURL = blob => {
      const url = originalCreateUrl(blob);
      uiBlobs.set(url, blob);
      return url;
    };
    URL.revokeObjectURL = url => {
      uiBlobs.delete(url);
      originalRevokeUrl(url);
    };
    window.NativeWorker = Worker;
    window.Worker = class extends NativeWorker {
      postMessage(data) {
        if (this === worker && data.operation !== 'inspect') {
          window.uiRequests.push(structuredClone(data.options));
          window.lastCompletion = this.onmessage;
        }
        return super.postMessage(data);
      }
    };
  `);
  const generate = (json, expected, args) => {
    const count = driver.execute_script('return uiRequests.length');
    submit(json, expected.format);
    wait(() => status().startsWith('Generated in'), 'advanced UI generation', 30000);
    assert.equal(driver.execute_script('return uiRequests.length'), count + 1);
    assert.deepEqual(driver.execute_script('return uiRequests.at(-1)'), expected);
    driver.execute_script(`
      window.uiContent = null;
      uiBlobs.get(document.getElementById('download').href).text().then(text => { window.uiContent = text; });
    `);
    const content = wait(() => driver.execute_script('return window.uiContent'), 'UI result content');
    assert.equal(content, native(json, args), 'UI/native exact parity');
    assert.equal(content, run(json, expected).content, 'UI/Wasm exact parity');
  };
  const types = ['aws_ssm_parameter', 'aws_cloudwatch_metric_alarm', 'aws_iam_role', 'aws_iam_user', 'aws_iam_group', 'aws_iam_openid_connect_provider', 'aws_iam_saml_provider'];
  assert.deepEqual(driver.execute_script('return Array.from(document.querySelectorAll("#spatial-types input"), input => input.value)'), types);
  for (const type of types) {
    const checkbox = find('#spatial-' + type);
    assert.equal(checkbox.accessible_name, type);
    checkbox.send_keys(Keys.SPACE);
    assert.deepEqual(options().spatial_types, [type]);
    assert.equal(driver.execute_script('return document.activeElement.id'), 'spatial-' + type);
    reset();
    assert.deepEqual(options(), defaults);
  }
  for (const fixture of ['iam', 'identity-providers', 'ssm', 'cloudwatch']) {
    const json = fixture === 'cloudwatch' ? JSON.stringify({format_version: '1.2', resource_changes: [
      {address: 'aws_db_instance.database', type: 'aws_db_instance',
        change: {actions: ['create'], after: {identifier: 'PRIVATE_DATABASE'}}},
      {address: 'aws_cloudwatch_metric_alarm.cpu', type: 'aws_cloudwatch_metric_alarm',
        change: {actions: ['update'], after: {namespace: 'AWS/RDS', metric_name: 'CPUUtilization', dimensions: {DBInstanceIdentifier: 'PRIVATE_DATABASE'}}}},
    ], configuration: {root_module: {resources: [
      {address: 'aws_cloudwatch_metric_alarm.cpu', expressions: {dimensions: {references: ['aws_db_instance.database.id']}}},
    ]}}}) : readFileSync(join(__dirname, `../fixtures/${fixture}-plan.json`), 'utf8');
    for (const selected of [[], [types[fixture === 'ssm' ? 0 : fixture === 'cloudwatch' ? 1 : 2]], types]) {
      reset();
      selected.forEach(type => set('spatial-' + type, true));
      for (const format of ['svg', 'html']) {
        generate(json, {...defaults, format, spatial_types: selected},
          ['--format', format, ...selected.flatMap(type => ['--spatial-type', type])]);
      }
    }
  }
  reset();
  for (const format of ['svg', 'html']) {
    set('format', format);
    set('address-format', 'terraform');
    set('icon-mode', 'service-name');
    set('hide-legend', true);
    set('changed', true);
    set('focus', 'aws_ecs_service.app');
    assert.equal(find('#focus-depth').is_enabled(), true);
    for (const depth of ['', '0', '2', '4294967295']) {
      set('focus-depth', depth);
      generate(small, {
        ...defaults, format, address_format: 'terraform', icon_mode: 'service-name',
        hide_relationship_legend: true, changed_only: true, focus: 'aws_ecs_service.app',
        ...(depth === '' ? {} : {focus_depth: Number(depth)}),
      }, ['--format', format, '--address-format', 'terraform', '--icon-mode', 'service-name',
        '--hide-relationship-legend', '--changed-only', '--focus', 'aws_ecs_service.app',
        ...(depth === '' ? [] : ['--focus-depth', depth])]);
    }
  }
  for (const depth of ['-1', '1.5', '1e2', ' ', 'secret', '4294967296']) {
    set('focus-depth', depth);
    const count = driver.execute_script('return uiRequests.length');
    submit(small, 'svg');
    assert.equal(driver.execute_script('return uiRequests.length'), count);
    assert.equal(find('#focus-depth').get_attribute('aria-invalid'), 'true');
    assert.equal(driver.execute_script('return document.activeElement.id'), 'focus-depth');
    assert.ok(find('#settings-error').text.startsWith('Focus depth must'));
    assert.ok(!find('#settings-error').text.includes('secret'));
    assert.equal(find('#focus-depth').get_attribute('value'), depth, 'Invalid input is not corrected');
  }
  set('focus', '');
  assert.equal(find('#focus-depth').get_attribute('aria-describedby'), 'depth-help');
  assert.equal(find('#focus-depth').is_enabled(), false);
  assert.ok(!('focus_depth' in options()));
  set('focus', 'aws_ecs_service.app');
  assert.equal(find('#focus-depth').get_attribute('value'), '4294967296');
  reset();
  assert.deepEqual(options(), defaults);
  assert.equal(find('#focus-depth').get_attribute('value'), '');
  assert.equal(find('#settings-error').text, '');
  assert.equal(find('#focus-depth').is_enabled(), false);
  assert.equal(driver.execute_script("return document.getElementById('file').files.length"), 1);

  find('#add-module').send_keys(Keys.ENTER);
  assert.equal(driver.execute_script('return document.activeElement.closest("#modules") !== null'), true);
  assert.equal(driver.switch_to.active_element.accessible_name, 'Module selector');
  submit(small, 'svg');
  assert.equal(find('#settings-error').text, 'Enter a module selector or remove the empty row.');
  assert.equal(driver.execute_script('return document.activeElement.getAttribute("aria-invalid")'), 'true');
  assert.equal(find('#focus-depth').get_attribute('aria-describedby'), 'depth-help', 'Corrected depth does not describe a module error');
  const selectors = ['module.worker["a b,c.d\\"e"]', 'module.worker["other"]'];
  const json = JSON.stringify({
    format_version: '1.2',
    resource_changes: [...selectors, 'module.excluded'].map(module => ({
      address: `${module}.terraform_data.item`, module_address: module, type: 'terraform_data',
      change: {actions: ['create'], after: {input: 'private'}},
    })),
  });
  find('#modules input').send_keys(selectors[0]);
  assert.equal(find('#modules input').get_attribute('aria-describedby'), 'modules-help');
  set('focus', 'aws_ecs_service.app');
  set('focus-depth', '-1');
  submit(small, 'svg');
  assert.ok(find('#settings-error').text.startsWith('Focus depth must'));
  assert.equal(find('#modules input').get_attribute('aria-describedby'), 'modules-help', 'Corrected module does not describe a depth error');
  set('focus', '');
  set('focus-depth', '');
  find('#add-module').send_keys(Keys.SPACE);
  driver.switch_to.active_element.send_keys(selectors[1]);
  find('#add-module').click();
  driver.switch_to.active_element.send_keys(selectors[0]);
  for (const format of ['svg', 'html']) {
    generate(json, {...defaults, format, modules: [selectors[0], selectors[1], selectors[0]]},
      ['--format', format, '--modules', selectors[0], '--modules', selectors[1], '--modules', selectors[0]]);
  }
  find('#modules button').send_keys(Keys.ENTER);
  assert.deepEqual(options().modules, [selectors[1], selectors[0]]);
  assert.equal(driver.execute_script('return document.activeElement.value'), selectors[1]);
  find('#modules button').click();
  find('#modules button').send_keys(Keys.ENTER);
  assert.deepEqual(options().modules, []);
  assert.equal(driver.execute_script('return document.activeElement.id'), 'add-module');
  find('#add-module').click();
  driver.switch_to.active_element.send_keys('module.secret[');
  submit(small, 'svg');
  wait(() => status() === 'The requested selection could not be resolved.', 'safe module error');
  assert.ok(!status().includes('secret'));
  reset();
  set('focus', 'secret');
  submit(small, 'svg');
  wait(() => status() === 'The requested selection could not be resolved.', 'safe focus error');
  reset();
  generate(small, defaults, []);

  driver.execute_script(`
    window.completedHandler = lastCompletion;
    window.originalFileText = File.prototype.text;
    File.prototype.text = function() {
      const file = this;
      return new Promise(resolve => { window.finishRead = () => resolve(originalFileText.call(file)); });
    };
  `);
  submit(small, 'svg');
  driver.execute_script(`
    document.getElementById('spatial-aws_iam_role').checked = true;
    finishRead();
  `);
  wait(() => status().startsWith('Generated in'), 'snapshot taken before file read');
  assert.deepEqual(driver.execute_script('return uiRequests.at(-1)'), defaults);
  reset();
  const requestCount = driver.execute_script('return uiRequests.length');
  submit(small, 'svg');
  set('spatial-aws_iam_role', true);
  driver.execute_script('finishRead();');
  assert.equal(driver.execute_script('return uiRequests.length'), requestCount, 'Settings invalidate pending file reads');
  assert.equal(driver.execute_script("return document.getElementById('download').hidden"), true);
  assert.equal(find('#cancel').is_enabled(), false);
  driver.execute_script(`
    completedHandler({data: {ok: true, content: 'STALE', diagnostics: [], elapsed_ms: 0}});
  `);
  assert.ok(status().startsWith('Settings changed.'));
  assert.equal(driver.execute_script("return document.getElementById('download').hidden"), true);
  for (const action of ['cancel', 'reset-settings']) {
    submit(small, 'svg');
    find('#' + action).click();
    driver.execute_script('finishRead();');
    assert.equal(driver.execute_script('return uiRequests.length'), requestCount);
  }
  submit(small, 'svg');
  driver.execute_script(`
    const transfer = new DataTransfer();
    transfer.items.add(new File(['{}'], 'replacement.json'));
    const finishOldRead = finishRead;
    document.getElementById('file').files = transfer.files;
    document.getElementById('file').dispatchEvent(new Event('change', {bubbles: true}));
    finishOldRead();
  `);
  assert.equal(driver.execute_script('return uiRequests.length'), requestCount, 'File replacement invalidates pending reads');
  driver.execute_script('File.prototype.text = originalFileText;');
  generate(small, defaults, []);
  reset();
  assert.equal(driver.execute_script('return worker === undefined'), true);
  assert.equal(driver.execute_script("return document.getElementById('download').hasAttribute('href')"), false);
  assert.equal(driver.execute_script("return document.getElementById('preview').hasAttribute('src')"), false);
  assert.equal(driver.execute_script('return uiBlobs.size'), 0, 'Reset revokes result and preview URLs');
  driver.execute_script(`
    window.Worker = NativeWorker;
    URL.createObjectURL = originalCreateUrl;
    URL.revokeObjectURL = originalRevokeUrl;
  `);
};

module.exports.checkStartupFailure = function({driver, url, status}) {
  for (const source of [
    'window.fetch = () => Promise.reject(new Error("test asset failure"));',
    'WebAssembly.compile = () => Promise.reject(new Error("test compile failure"));',
  ]) {
    const {identifier} = driver.execute_cdp_cmd('Page.addScriptToEvaluateOnNewDocument', {source});
    try {
      driver.get(url);
      const failure = 'Could not load the generator. Reload while connected.';
      wait(() => status() === failure, 'startup failure');
      const assertFailure = () => {
        assert.equal(status(), failure);
        assert.equal(driver.find_element(By.CSS_SELECTOR, '#generate').is_enabled(), false);
      };
      driver.find_element(By.CSS_SELECTOR, '#focus').send_keys('example');
      assertFailure();
      driver.execute_script(`
        const transfer = new DataTransfer();
        transfer.items.add(new File(['{}'], 'plan.json'));
        document.getElementById('file').files = transfer.files;
        document.getElementById('file').dispatchEvent(new Event('change', {bubbles: true}));
      `);
      assertFailure();
      for (const selector of ['#add-module', '#modules button', '#reset-settings']) {
        driver.find_element(By.CSS_SELECTOR, selector).click();
        assertFailure();
      }
    } finally {
      driver.execute_cdp_cmd('Page.removeScriptToEvaluateOnNewDocument', {identifier});
    }
  }
};
