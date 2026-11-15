const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const {mkdtempSync, readFileSync, writeFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const {resolve, join} = require('node:path');
const {pathToFileURL} = require('node:url');
const {Driver, By, wait} = require('./webdriver.cjs');

const root = resolve(__dirname, '../..');
const binary = resolve(process.env.CARGO_TARGET_DIR || join(root, 'target'), 'release/planorama');
const fixtures = ['anonymization', 'accounts', 'cloudflare-applications', 'organization-deployments', 'deposed', 'drift', 'lifecycle', 'checks'];
const extra = ['sentinel', 'TOP_SECRET', 'future<&status', '198.51.100.0/24', '2001:db8:abcd::/48', '739185026412', '123456789012', '210987654321', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'];

function secrets(plan) {
  return [...new Set([...extra, ...Object.keys(plan.configuration?.provider_config || {}).filter(key => !['aws', 'cloudflare', 'terraform'].includes(key)), ...[...(plan.resource_changes || []), ...(plan.resource_drift || [])]
    .flatMap(resource => [resource.address, resource.previous_address, resource.deposed]).filter(Boolean)])];
}

function scan(text, values, context) {
  for (const value of values) {
    const hex = Buffer.from(value).toString('hex');
    const xml = value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
    for (const encoded of [value, xml, JSON.stringify(value).slice(1, -1), hex, Buffer.from(hex).toString('hex'), Buffer.from(value).toString('base64'), encodeURIComponent(value)]) {
      assert.ok(!text.includes(encoded), `${context}: leaked ${value}`);
    }
  }
}

function generate(fixture, format, anonymize, named) {
  const input = join(root, `tests/fixtures/${fixture}-plan.json`);
  const args = named ? ['--state', `first_environment_sentinel=${input}`, '--state', `second_environment_sentinel=${input}`] : [input];
  return execFileSync(binary, [...args, '--format', format, ...(anonymize ? ['--anonymize'] : []), '-o', '-'],
    {encoding: 'utf8', timeout: 120000, maxBuffer: 16 * 1024 * 1024});
}

async function main() {
  const temp = mkdtempSync(join(tmpdir(), 'planorama-privacy-audit-'));
  let driver;
  try {
    driver = await Driver.start();
    driver.execute_cdp_cmd('Network.enable', {});
    driver.execute_cdp_cmd('Network.emulateNetworkConditions', {offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0});
    const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
    const search = text => {
      const field = find('#search');
      field.clear(); field.send_keys(text);
      return driver.find_elements(By.CSS_SELECTOR, '#results button');
    };
    for (const fixture of fixtures) {
      const plan = JSON.parse(readFileSync(join(root, `tests/fixtures/${fixture}-plan.json`), 'utf8'));
      const values = secrets(plan);
      const original = plan.resource_changes[0].address;
      for (const named of [false, true]) {
        for (const format of ['svg', 'html']) {
          const ordinary = generate(fixture, format, false, named);
          assert.ok(ordinary.includes(original), `${fixture}: ordinary mode must retain its resource identity`);
          if (fixture === 'cloudflare-applications') assert.ok(ordinary.includes('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'), 'Cloudflare scope positive control');
          const anonymous = generate(fixture, format, true, named);
          scan(anonymous, values, `${fixture}/${format}/${named}`);
          if (format !== 'html') continue;
          const path = join(temp, 'diagram.html');
          writeFileSync(path, ordinary);
          driver.get(pathToFileURL(path).href);
          wait(() => find('#result-count').text.includes('resources'), 'ordinary viewer initializes');
          assert.ok(search(original).length, 'ordinary search regression guard');
          writeFileSync(path, anonymous);
          driver.get(pathToFileURL(path).href);
          wait(() => find('#result-count').text.includes('resources'), 'anonymous viewer initializes');
          assert.equal(search(original).length, 0, 'original names must not be searchable');
          const resources = driver.execute_script("return JSON.parse(document.getElementById('viewer-data').textContent)");
          assert.ok(resources.length);
          assert.equal(new Set(resources.map(resource => resource.id)).size, resources.length);
          const target = resources[0];
          const query = (target.state ? target.state + ':' : '') + target.qualified;
          const matches = search(query);
          assert.ok(matches.length, 'pseudonym remains searchable');
          matches[0].click();
          assert.equal(find('#viewport [data-viewer-selected]').get_attribute('id'), target.id);
          const broken = driver.execute_script(`return [...document.querySelectorAll('#viewport a[href^="#"]')]
            .map(link => link.getAttribute('href').slice(1)).filter(id => !document.getElementById(id))`);
          assert.deepEqual(broken, [], 'all anonymous links resolve');
          const link = driver.find_elements(By.CSS_SELECTOR, '#viewport a[href^="#"]')[0];
          if (link) {
            const id = link.get_attribute('href').slice(1);
            driver.execute_script('arguments[0].dispatchEvent(new MouseEvent("click", {bubbles:true,cancelable:true}))', link);
            assert.equal(find('#viewport [data-viewer-selected]').get_attribute('id'), id);
          }
          find('#search').clear();
          scan(driver.execute_script('return document.documentElement.outerHTML'), values, 'live DOM after navigation');
          for (const kind of ['viewport', 'neighborhood']) {
            driver.execute_script("window.auditText=null; URL.createObjectURL=blob=>{window.auditBlob=blob;return 'blob:audit';}; HTMLAnchorElement.prototype.click=function(){};");
            find('#export-' + kind).click();
            driver.execute_script('window.auditBlob.text().then(text=>window.auditText=text)');
            wait(() => driver.execute_script('return !!window.auditText'), 'viewer export');
            scan(driver.execute_script('return window.auditText'), values, `${fixture}: ${kind} SVG export`);
          }
          assert.deepEqual(driver.get_log('browser').filter(entry => entry.level === 'SEVERE'), []);
          const requests = driver.get_log('performance').map(entry => JSON.parse(entry.message).message);
          assert.deepEqual(requests.filter(entry => entry.method === 'Network.requestWillBeSent' && /^https?:/.test(entry.params.request.url)), []);
        }
      }
      console.log(`PASS privacy bytes, DOM, search, links and exports: ${fixture}`);
    }
  } finally {
    try { if (driver) await driver.quit(); }
    finally { rmSync(temp, {recursive: true, force: true}); }
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
