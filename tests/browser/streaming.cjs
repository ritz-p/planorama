const assert = require('node:assert/strict');
const {mkdtempSync, rmSync} = require('node:fs');
const {join} = require('node:path');
const {tmpdir} = require('node:os');
const {Driver} = require('./webdriver.cjs');
const {ViewerTests} = require('./viewer.cjs');

async function main() {
    const temp = mkdtempSync(join(tmpdir(), 'planorama-streaming-'));
    let driver;
    try {
        driver = await Driver.start();
        driver.execute_cdp_cmd('Network.enable', {});
        driver.execute_cdp_cmd('Network.emulateNetworkConditions', {
            offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0,
        });
        const viewer = new ViewerTests(driver, temp);
        viewer.load('--state', 'one=tests/fixtures/kinesis-plan.json',
            '--state', 'two=tests/fixtures/kinesis-plan.json');
        const ids = new Set();
        for (const state of ['one', 'two']) {
            for (const address of ['aws_kinesis_stream.jobs', 'aws_lambda_event_source_mapping.jobs']) {
                ids.add(viewer.choose(`${state}:${address}`));
            }
        }
        assert.equal(ids.size, 4);
        assert.equal(driver.execute_script(`return [...document.querySelectorAll('#viewport a[href]')]
            .filter(a => !document.getElementById(a.getAttribute('href').slice(1))).length`), 0);
        viewer.check_logs();
        console.log('PASS streaming offline search, selection and state-qualified links');
    } finally {
        try { if (driver) await driver.quit(); }
        finally { rmSync(temp, {recursive: true, force: true}); }
    }
}
main().catch(error => {console.error(error); process.exitCode = 1;});
