const assert = require('node:assert/strict');
const {fork} = require('node:child_process');
const {readFileSync, mkdtempSync, rmSync, existsSync} = require('node:fs');
const {join, resolve} = require('node:path');
const {tmpdir} = require('node:os');
const {Driver, By, wait} = require('./webdriver.cjs');

const root = resolve(__dirname, '../..');
const binary = join(process.env.CARGO_TARGET_DIR || join(root, 'target'), 'release/planorama');
const nativeReference = require('../support/native-reference.cjs')(binary);
const small = readFileSync(join(root, 'tests/fixtures/viewer-plan.json'), 'utf8');
const server = fork(join(root, 'scripts/serve-browser.cjs'), [], {env: {...process.env, PORT: '0'}, stdio: ['ignore', 'inherit', 'inherit', 'ipc']});
const temp = mkdtempSync(join(tmpdir(), 'planorama-wasm-'));

async function main() {
  const port = await new Promise((resolve, reject) => {
    server.once('message', message => resolve(message.port));
    server.once('error', reject);
    server.once('exit', () => reject(new Error('Static server exited')));
  });
  const driver = await Driver.start();
  const run = (json, options = {}, operation = 'generate') => {
    driver.execute_script(`
      window.testResult = null;
      window.progressStages = [];
      const w = new Worker(assets.url);
      w.onmessage = ({data}) => {
        if (data.type === 'progress') { window.progressStages.push(data); return; }
        window.testResult = data; w.terminate();
      };
      w.onerror = () => { window.testResult = {ok: false, trap: true}; w.terminate(); };
      w.postMessage({module: assets.module, json: arguments[0], options: arguments[1], operation: arguments[2]});
    `, json, options, operation);
    return wait(() => driver.execute_script('return window.testResult ? [window.testResult] : null'), 'Wasm result', 180000)[0];
  };
  const status = () => driver.execute_script("return document.getElementById('status').textContent");
  const submit = (json, format) => driver.execute_script(`
    const transfer = new DataTransfer();
    transfer.items.add(new File([arguments[0]], 'plan.json', {type: 'application/json'}));
    document.getElementById('file').files = transfer.files;
    document.getElementById('format').value = arguments[1];
    document.getElementById('generate-form').requestSubmit();
  `, json, format);
  try {
    require('./wasm-options.cjs').checkStartupFailure({driver, url: `http://127.0.0.1:${port}/`, status});
    driver.get(`http://127.0.0.1:${port}/`);
    wait(() => status().startsWith('Ready.'), 'asset loading', 30000);
    driver.execute_cdp_cmd('Network.enable', {});
    driver.execute_cdp_cmd('Network.emulateNetworkConditions', {offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0});
    driver.get_log('performance');
    require('./wasm-named.cjs')({driver, root, binary});
    require('./wasm-modules.cjs')({
      driver, small, run, status,
      native: nativeReference,
    });
    require('./wasm-resources.cjs')({driver, run,
      native: nativeReference,
    });
    require('./wasm-options.cjs')({
      driver, small, run, submit, status,
      native: nativeReference,
    });
    for (const [options, args] of [
      [{}, []],
      [{format: 'html'}, ['--format', 'html']],
      [{changed_only: true}, ['--changed-only']],
      [{focus: 'aws_ecs_service.app', focus_depth: 0, address_format: 'terraform'}, ['--focus', 'aws_ecs_service.app', '--focus-depth', '0', '--address-format', 'terraform']],
      [{modules: ['module.worker'], icon_mode: 'service-name', hide_relationship_legend: true}, ['--modules', 'module.worker', '--icon-mode', 'service-name', '--hide-relationship-legend']],
    ]) {
      const result = run(small, options);
      assert.equal(result.ok, true, JSON.stringify(result));
      const native = nativeReference(small, args);
      assert.equal(result.content, native, `Native/Wasm exact parity: ${JSON.stringify(options)}`);
      console.log(JSON.stringify({fixture: 'viewer-plan', options, elapsed_ms: result.elapsed_ms, memory_bytes: result.memory_bytes, output_bytes: Buffer.byteLength(result.content)}));
    }
    const aliases = readFileSync(join(root, 'tests/fixtures/aws-aliases-plan.json'), 'utf8');
    for (const fixture of ['dynamodb-plan', 'elasticache-configuration-plan']) {
      const input = readFileSync(join(root, 'tests/fixtures/'+fixture+'.json'), 'utf8');
      for (const format of ['svg', 'html']) {
        const result = run(input, {format});
        assert.equal(result.ok, true, JSON.stringify(result));
        assert.equal(result.content, nativeReference(input, ['--format', format]), fixture+' native/Wasm '+format);
        assert.ok(!result.content.includes('PRIVATE_'));
      }
    }
    for (const format of ['svg', 'html']) {
      for (const icon_mode of ['service-name']) {
        const result = run(aliases, {format, icon_mode});
        assert.equal(result.ok, true, JSON.stringify(result));
        const native = nativeReference(aliases, ['--format', format, '--icon-mode', icon_mode]);
        assert.equal(result.content, native, `AWS aliases native/Wasm ${format}/${icon_mode}`);
        assert.ok(result.content.includes('aws_alb.web'));
      }
    }
    for (const [json, options, code] of [
      ['secret{', {}, 'invalid_json'],
      ['{"format_version":"secret"}', {}, 'unsupported_input'],
      ['{"format_version":"1.0","values":{}}', {}, 'unsupported_input'],
      [small, {provider_plan: 'secret'}, 'invalid_options'],
      [small, {omit_relationship_legend: 'true'}, 'invalid_options'],
      [small, {states: ['secret']}, 'invalid_options'],
      [small, {focus: 'secret'}, 'selection_failed'],
      [small, {focus_depth: 2}, 'invalid_options'],
      [small, {spatial_types: ['aws_iam_policy']}, 'invalid_options'],
      [' '.repeat(32 * 1024 * 1024 + 1), {}, 'input_too_large'],
    ]) {
      const result = run(json, options);
      assert.equal(result.error?.code, code);
      assert.ok(!JSON.stringify(result).includes('secret'));
    }
    for (const fixture of ['iam', 'identity-providers', 'ssm']) {
      const json = readFileSync(join(root, `tests/fixtures/${fixture}-plan.json`), 'utf8');
      const spatial_types = ['aws_ssm_parameter', 'aws_cloudwatch_metric_alarm', 'aws_iam_role', 'aws_iam_user', 'aws_iam_group', 'aws_iam_openid_connect_provider', 'aws_iam_saml_provider'];
      for (const format of ['svg', 'html']) {
        const result = run(json, {format, spatial_types});
        assert.equal(result.ok, true, JSON.stringify(result));
        const native = nativeReference(json, ['--format', format, ...spatial_types.flatMap(type => ['--spatial-type', type])]);
        assert.equal(result.content, native, `Spatial native/Wasm parity: ${fixture}/${format}`);
      }
    }
    for (const fixture of ['aws-basic', 'service-labels']) {
      const basic = readFileSync(join(root, `tests/fixtures/${fixture}-plan.json`), 'utf8');
      for (const format of ['svg', 'html']) {
        for (const icon_mode of ['service-name']) {
          const result = run(basic, {format, icon_mode});
          assert.equal(result.ok, true, JSON.stringify(result));
          const native = nativeReference(basic, ['--format', format, '--icon-mode', icon_mode]);
          assert.equal(result.content, native, `Resource labels native/Wasm parity: ${fixture}/${format}/${icon_mode}`);
          assert.ok(!result.content.includes('PRIVATE_'));
        }
      }
    }
    submit('secret{', 'svg');
    wait(() => status() === 'Input is not valid plan JSON.', 'UI error');
    driver.execute_cdp_cmd('Browser.setDownloadBehavior', {behavior: 'allow', downloadPath: temp});
    for (const format of ['svg', 'html']) {
      driver.find_element(By.CSS_SELECTOR, '#file').send_keys(join(root, 'tests/fixtures/viewer-plan.json'));
      driver.execute_script("document.getElementById('format').value = arguments[0]; document.getElementById('generate-form').requestSubmit();", format);
      wait(() => status().startsWith('Generated in'), 'UI generation');
      driver.find_element(By.CSS_SELECTOR, '#download').click();
      wait(() => existsSync(join(temp, 'diagram.' + format)), 'offline download');
      const saved = readFileSync(join(temp, 'diagram.' + format), 'utf8');
      assert.equal(saved, run(small, {format}).content);
      driver.command('POST', '/frame', {id: driver.find_element(By.CSS_SELECTOR, '#preview')});
      wait(() => driver.execute_script('return !!document.querySelector("svg")'), 'preview loaded');
      if (format === 'html') {
        wait(() => driver.execute_script('return document.getElementById("result-count").textContent.includes("resources")'), 'interactive HTML preview');
        driver.find_element(By.CSS_SELECTOR, '#search').send_keys('aws_ecs_service.app');
        wait(() => driver.execute_script('return document.querySelectorAll("#results button").length === 1'), 'preview search');
      }
      assert.equal(driver.execute_script('try { parent.document.body; return true; } catch { return false; }'), false);
      driver.command('POST', '/frame', {id: null});
    }
    const hostile = JSON.stringify({format_version: '1.2', resource_changes: [{
      address: 'terraform_data.<img src="https://example.invalid/leak" onerror="parent.pwned=1"><script>parent.pwned=1</script>',
      type: 'terraform_data', change: {actions: ['create'], after: {input: 'PRIVATE_VALUE_SENTINEL'}},
    }]});
    for (const format of ['svg', 'html']) {
      const result = run(hostile, {format});
      assert.equal(result.ok, true);
      assert.ok(!result.content.includes('PRIVATE_VALUE_SENTINEL'));
      submit(hostile, format);
      wait(() => status().startsWith('Generated in'), 'hostile preview');
      driver.command('POST', '/frame', {id: driver.find_element(By.CSS_SELECTOR, '#preview')});
      assert.equal(driver.execute_script('return document.querySelectorAll("img[onerror], script[src]").length'), 0);
      driver.command('POST', '/frame', {id: null});
      assert.equal(driver.execute_script('return window.pwned || null'), null);
    }
    const largePath = join(root, 'examples/terraform-large/plan.json');
    const large = readFileSync(largePath, 'utf8');
    submit(large, 'svg');
    wait(() => driver.execute_script('return !!worker'), 'worker start');
    driver.execute_script(`
      window.oldCompletion = worker.onmessage;
      document.getElementById('spatial-aws_iam_role').checked = true;
      document.getElementById('spatial-aws_iam_role').dispatchEvent(new Event('input', {bubbles: true}));
    `);
    assert.equal(driver.execute_script('return worker === undefined'), true);
    assert.ok(status().startsWith('Settings changed.'));
    assert.equal(driver.execute_script('return elapsedTimer === undefined'), true);
    driver.find_element(By.CSS_SELECTOR, '#reset-settings').click();
    submit(large, 'svg');
    wait(() => driver.execute_script('return !!worker'), 'worker before reset');
    driver.find_element(By.CSS_SELECTOR, '#reset-settings').click();
    assert.equal(driver.execute_script('return worker === undefined'), true);
    assert.ok(status().startsWith('Settings reset'));
    submit(large, 'svg');
    wait(() => driver.execute_script('return !!worker'), 'worker before file replacement');
    driver.execute_script("window.replacedWorker = worker; document.getElementById('file').dispatchEvent(new Event('change', {bubbles: true}));");
    assert.equal(driver.execute_script('return worker !== replacedWorker'), true);
    submit(large, 'svg');
    wait(() => driver.execute_script('return !!worker'), 'worker before error');
    driver.execute_script("worker.onerror({preventDefault() {}});");
    assert.equal(status(), 'Diagram generation failed. Try a smaller plan.');
    assert.equal(driver.execute_script('return worker === undefined'), true);
    assert.equal(driver.execute_script('return elapsedTimer === undefined'), true);
    submit(large, 'svg');
    wait(() => driver.execute_script('return !!worker'), 'worker before cancel');
    wait(() => status() === 'Routing connections…', 'large fixture routing stage', 180000);
    assert.equal(driver.execute_script("return document.getElementById('cancel').disabled"), false);
    assert.match(driver.execute_script("return document.getElementById('elapsed').textContent"), /^Elapsed: \d+ s$/);
    const routingStatus = status();
    driver.execute_script("worker.onmessage({data: {type: 'progress', request: serial - 1, stage: 'output'}});");
    assert.equal(status(), routingStatus, 'Wrong request progress is discarded');
    driver.find_element(By.CSS_SELECTOR, '#cancel').click();
    assert.ok(status().startsWith('Cancelled.'));
    assert.equal(driver.execute_script('return elapsedTimer === undefined'), true);
    submit(small, 'svg');
    wait(() => status().startsWith('Generated in'), 'retry after cancellation');
    const currentStatus = status();
    driver.execute_script("oldCompletion({data: {ok: false, error: {message: 'STALE'}}});");
    driver.execute_script("oldCompletion({data: {type: 'progress', request: serial, stage: 'routing'}});");
    assert.equal(status(), currentStatus, 'Stale completion cannot replace a newer result');
    assert.equal(driver.execute_script('return elapsedTimer === undefined'), true);
    const largeResult = run(large);
    assert.equal(largeResult.ok, true);
    assert.deepEqual(driver.execute_script('return progressStages.map(event => event.stage)'), ['parsing', 'selection', 'placement', 'routing', 'output']);
    assert.equal(driver.execute_script("return progressStages.every(event => Object.keys(event).sort().join(',') === 'request,stage,type')"), true);
    const native = nativeReference(large, [], {timeout: 180000, maxBuffer: 64 * 1024 * 1024});
    assert.equal(largeResult.content, native, 'Large native/Wasm exact parity');
    console.log(JSON.stringify({fixture: 'terraform-large', input_bytes: Buffer.byteLength(large), elapsed_ms: largeResult.elapsed_ms, memory_bytes: largeResult.memory_bytes, output_bytes: Buffer.byteLength(largeResult.content)}));
    const requests = driver.get_log('performance').map(entry => JSON.parse(entry.message).message)
      .filter(entry => entry.method === 'Network.requestWillBeSent' && /^https?:/.test(entry.params.request.url));
    assert.deepEqual(requests, [], 'No HTTP requests after assets load, including after worker cancellation');
    assert.equal(driver.execute_script('return localStorage.length + sessionStorage.length'), 0);
    driver.execute_script('window.databaseNames = null; indexedDB.databases().then(dbs => { window.databaseNames = dbs.map(db => db.name); });');
    assert.deepEqual(wait(() => driver.execute_script('return window.databaseNames'), 'IndexedDB check'), []);
    const errors = driver.get_log('browser').filter(entry => entry.level === 'SEVERE');
    assert.deepEqual(errors, []);
    console.log('Browser Wasm: parity, errors, isolated previews, downloads, cancellation and offline checks passed.');
  } finally {
    console.log({nativeReference: nativeReference.stats});
    await driver.quit();
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => {
  server.kill();
  rmSync(temp, {recursive: true, force: true});
});
