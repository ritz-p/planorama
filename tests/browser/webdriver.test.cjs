const { test } = require('node:test');
const assert = require('node:assert/strict');
const { mkdtempSync, writeFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const { pathToFileURL } = require('node:url');
const { Driver, wait } = require('./webdriver.cjs');
const { ViewerTests } = require('./viewer.cjs');

test('offline monitor detects attempted HTTP(S) and severe console errors; cleanup follows failure', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-monitor-'));
  let driver;
  try {
    driver = await Driver.start();
    driver.execute_cdp_cmd('Network.enable', {});
    driver.execute_cdp_cmd('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
    const file = join(directory, 'probe.html');
    writeFileSync(file, '<!doctype html><title>Monitor probe</title>');
    driver.get(pathToFileURL(file).href);
    const suite = new ViewerTests(driver, directory);
    suite.check_logs();
    driver.execute_script('console.error("deliberate severe error")');
    assert.throws(() => suite.check_logs(), /deliberate severe error/);
    for (const protocol of ['http', 'https']) {
      driver.execute_script('window.probeDone=false; fetch(arguments[0]).catch(()=>{}).finally(()=>window.probeDone=true)', protocol + '://example.invalid/probe');
      wait(() => driver.execute_script('return window.probeDone'), 'offline request rejection');
      driver.get_log('browser');
      assert.throws(() => suite.check_logs(), /example.invalid/);
    }
    assert.throws(() => wait(() => false, 'deliberate condition failure', 50), /Timed out waiting/);
  } finally {
    try { if (driver) await driver.quit(); }
    finally { rmSync(directory, { recursive: true, force: true }); }
  }
  assert.ok(driver.process.exitCode !== null || driver.process.signalCode !== null);
});
