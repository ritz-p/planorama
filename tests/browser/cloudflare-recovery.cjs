const assert = require('node:assert/strict');
const {execFileSync} = require('node:child_process');
const {mkdirSync, mkdtempSync, readFileSync, rmSync} = require('node:fs');
const {resolve, join} = require('node:path');
const {pathToFileURL} = require('node:url');
const {Driver, By, wait} = require('./webdriver.cjs');

async function main() {
  const root = resolve(__dirname, '../..');
  const output = join(root, 'output');
  mkdirSync(output, {recursive: true});
  const temp = mkdtempSync(join(output, 'cloudflare-recovery-'));
  const binary = resolve(process.env.CARGO_TARGET_DIR || join(root, 'target'), 'release/planorama');
  const fixture = join(root, 'tests/fixtures/cloudflare-imported-plan.json');
  let driver;
  try {
    driver = await Driver.start();
    for (const named of [false, true]) {
      const path = join(temp, 'diagram.html');
      execFileSync(binary, [...(named ? ['--state', `one=${fixture}`, '--state', `two=${fixture}`] : [fixture]), '--cloudflare-value-matches', '--no-omit-relationship-legend', '--format', 'html', '-o', path]);
      driver.get(pathToFileURL(path).href);
      wait(() => driver.execute_script("return document.querySelector('#result-count')?.textContent.includes('resources')"), 'viewer ready');
      const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#viewport [data-evidence="configured-value"]').length, named ? 8 : 4);
      assert.ok(find('#viewport').text.includes('Purple dotted lines'));
      find('#search').send_keys((named ? 'one:' : '') + 'cloudflare_workers_route.api');
      find('#results button').click();
      assert.equal(find('[data-viewer-selected]').get_attribute('data-terraform-address'), 'cloudflare_workers_route.api');
      driver.execute_script("window.recoveredBlob=null; URL.createObjectURL=blob=>{window.recoveredBlob=blob;return 'blob:recovery';}; HTMLAnchorElement.prototype.click=function(){};");
      find('#export-neighborhood').click();
      assert.ok(driver.execute_script('return window.recoveredBlob instanceof Blob'));
      driver.execute_script('window.recoveredBlob.text().then(text=>window.recoveredText=text)');
      wait(() => driver.execute_script('return !!window.recoveredText'), 'neighborhood download');
      const exported = driver.execute_script('return window.recoveredText');
      assert.ok(exported.includes('configured-value'));
      assert.ok(exported.includes('typed-configured-value'));
      assert.ok(!exported.includes('PRIVATE_IMPORT_ID'));
      assert.ok(readFileSync(path, 'utf8').includes('not verified Terraform provenance'));
      assert.deepEqual(driver.get_log('browser').filter(entry => entry.level === 'SEVERE'), []);
    }
    console.log('Cloudflare configured-value viewer navigation and neighborhood export passed');
  } finally {
    try { if (driver) await driver.quit(); }
    finally { rmSync(temp, {recursive: true, force: true}); }
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
