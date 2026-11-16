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
  const temp = mkdtempSync(join(output, 'cloudflare-selection-'));
  const binary = resolve(process.env.CARGO_TARGET_DIR || join(root, 'target'), 'release/planorama');
  const fixture = join(root, 'tests/fixtures/cloudflare-administration-plan.json');
  let driver;
  try {
    driver = await Driver.start();
    for (const named of [false, true]) {
      const path = join(temp, 'diagram.html');
      execFileSync(binary, [...(named ? ['--state', `one=${fixture}`, '--state', `two=${fixture}`] : [fixture]), '--cloudflare-groups', '--format', 'html', '-o', path]);
      driver.get(pathToFileURL(path).href);
      wait(() => driver.execute_script("return document.querySelector('#result-count')?.textContent.includes('resources')"), 'viewer ready');
      const find = selector => driver.find_element(By.CSS_SELECTOR, selector);
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#viewport [data-cloudflare-management-member]').length, named ? 18 : 9);
      driver.execute_script("const a=document.querySelector('#search-cloudflare-account');a.value=JSON.stringify('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');a.dispatchEvent(new Event('change'))");
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#results button').length, named ? 12 : 6);
      driver.execute_script("const z=document.querySelector('#search-cloudflare-zone');z.value=JSON.stringify('bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb');z.dispatchEvent(new Event('change'))");
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#results button').length, named ? 4 : 2);
      find('#search-reset').click();
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#results button').length, named ? 20 : 10);
      driver.execute_script("const a=document.querySelector('#search-cloudflare-account');a.value='null';a.dispatchEvent(new Event('change'))");
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '#results button').length, named ? 4 : 2);
      assert.ok(!driver.execute_script("return document.querySelector('#results').textContent.includes('aws_s3_bucket')"));
      find('#search-reset').click();
      driver.execute_script("document.querySelector('#viewport [data-cloudflare-management-member] text').dispatchEvent(new MouseEvent('click', {bubbles:true,cancelable:true}))");
      assert.equal(driver.find_elements(By.CSS_SELECTOR, '[data-viewer-selected]').length, 1);
      assert.deepEqual(driver.get_log('browser').filter(entry => entry.level === 'SEVERE'), []);
    }
    console.log('Cloudflare scope viewer search and administrative index navigation passed');
  } finally {
    try { if (driver) await driver.quit(); }
    finally { rmSync(temp, {recursive: true, force: true}); }
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
