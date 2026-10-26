const assert = require('node:assert/strict');
const fs = require('node:fs');
const {execFileSync, fork} = require('node:child_process');
const {createHash} = require('node:crypto');
const {join, resolve} = require('node:path');
const {tmpdir} = require('node:os');
const {Driver, wait} = require('./webdriver.cjs');

async function main() {
  const version = fs.readFileSync('Cargo.toml', 'utf8').match(/^version = "([^"]+)"/m)[1];
  const archive = `planorama-${version}-wasm-no-artwork.tar.gz`;
  const temp = fs.mkdtempSync(join(tmpdir(), 'planorama-engine-'));
  let driver;
  let server;
  try {
    const bytes = fs.readFileSync(`output/wasm-release/${archive}`);
    const future = new Date('2030-01-01T00:00:00Z');
    fs.utimesSync('output/wasm-engine/planorama.js', future, future);
    execFileSync(process.execPath, ['scripts/package-wasm-engine.cjs']);
    assert.deepEqual(fs.readFileSync(`output/wasm-release/${archive}`), bytes, 'archive bytes must not depend on file timestamps');
    assert.equal(createHash('sha256').update(bytes).digest('hex'), fs.readFileSync(`output/wasm-release/${archive}.sha256`, 'utf8').split(' ')[0]);
    execFileSync('tar', ['-xzf', `output/wasm-release/${archive}`, '-C', temp]);
    assert.deepEqual(fs.readdirSync(temp).sort(), ['LICENSE', 'LICENSE-APACHE', 'LICENSE-MIT', 'THIRD-PARTY-NOTICES.txt', 'manifest.json', 'planorama.js', 'planorama_bg.wasm']);
    const manifest = JSON.parse(fs.readFileSync(join(temp, 'manifest.json')));
    assert.equal(manifest.version, version);
    const git = args => execFileSync('git', ['-c', `safe.directory=${process.cwd()}`, ...args], {encoding: 'utf8'});
    const revision = git(['rev-parse', 'HEAD']).trim();
    const dirty = git(['status', '--porcelain', '--untracked-files=normal']).length > 0;
    assert.equal(manifest.commit, revision + (dirty ? '-dirty' : ''));
    assert.equal(manifest.icon_artwork, false);
    assert.equal(manifest.memory.maximum_bytes, 4294967296);
    for (const [file, hash] of Object.entries(manifest.files)) {
      const content = fs.readFileSync(join(temp, file));
      assert.equal(createHash('sha256').update(content).digest('hex'), hash);
      assert.doesNotMatch(content.toString('utf8'), /<symbol\b|<image\b|data:image\//i);
    }
    server = fork(__filename, ['--serve', temp, version], {stdio: ['ignore', 'inherit', 'inherit', 'ipc']});
    const port = await new Promise((resolve, reject) => {
      server.once('message', resolve);
      server.once('error', reject);
      server.once('exit', () => reject(new Error('Engine fixture server exited')));
    });
    driver = await Driver.start();
    driver.get(`http://127.0.0.1:${port}/`);
    wait(() => driver.execute_script('return window.ready === true;'), 'versioned engine initialization');
    assert.deepEqual(JSON.parse(driver.execute_script('return window.engine.memory_capabilities();')), manifest.memory);
    require('./memory-chromium.cjs')(driver, fs.readFileSync(join(temp, 'planorama.js'), 'utf8'), `/v${version}/planorama_bg.wasm`, fs.readFileSync('tests/fixtures/viewer-plan.json', 'utf8'));
    const binary = resolve(process.env.CARGO_TARGET_DIR || 'target', 'release/planorama');
    for (const fixture of ['viewer-plan', 'aws-basic-plan', 'terraform-plan']) {
      const json = fs.readFileSync(`tests/fixtures/${fixture}.json`, 'utf8');
      for (const format of ['svg', 'html']) {
        for (const icon_mode of [undefined, 'service-name']) {
          const result = JSON.parse(driver.execute_script('return window.engine.generate(arguments[0], arguments[1]);', json, JSON.stringify({format, icon_mode})));
          const native = execFileSync(binary, ['-', '-o', '-', '--format', format, '--icon-mode', 'service-name'], {input: json, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
          assert.equal(result.content, native, `${fixture}/${format}/${icon_mode}`);
          if (format === 'html') {
            const metadata = JSON.parse(result.content.match(/<script id="viewer-generation" type="application\/json">(.*?)<\/script>/s)[1]);
            assert.equal(metadata.generator.revision, manifest.commit);
          }
          assert.doesNotMatch(result.content, /<symbol\b|<image\b|data:image\/|planorama-icon-(aws|terraform)/i);
        }
      }
      const error = driver.execute_script('try { window.engine.generate(arguments[0], \'{"icon_mode":"icon"}\'); } catch(e) { return JSON.parse(e).code; }', json);
      assert.equal(error, 'invalid_options');
    }
    console.log('Extracted engine: checksums, browser generation, SVG/HTML parity and artwork exclusion passed');
  } finally {
    if (driver) await driver.quit();
    if (server) server.kill();
    fs.rmSync(temp, {recursive: true, force: true});
  }
}
if (process.argv[2] === '--serve') {
  const directory = process.argv[3];
  const prefix = `/v${process.argv[4]}/`;
  const server = require('node:http').createServer((request, response) => {
    if (request.url === '/') {
      response.setHeader('Content-Type', 'text/html');
      response.end(`<script src="${prefix}planorama.js"></script><script>window.engine = wasm_bindgen; engine({module_or_path: '${prefix}planorama_bg.wasm'}).then(() => window.ready = true);</script>`);
    } else if ([`${prefix}planorama.js`, `${prefix}planorama_bg.wasm`].includes(request.url)) {
      response.setHeader('Content-Type', request.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
      response.end(fs.readFileSync(join(directory, request.url.slice(prefix.length))));
    } else response.writeHead(404).end();
  });
  server.listen(0, '127.0.0.1', () => process.send(server.address().port));
} else main().catch(error => { console.error(error); process.exitCode = 1; });
