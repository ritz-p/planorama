const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const directory = 'output/wasm-engine';
const version = fs.readFileSync('Cargo.toml', 'utf8').match(/^version = "([^"]+)"/m)[1];
const engine = new Function(fs.readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
engine.initSync({module: fs.readFileSync(`${directory}/planorama_bg.wasm`)});
const html = JSON.parse(engine.generate('{"format_version":"1.2","resource_changes":[]}', '{"format":"html"}')).content;
const commit = JSON.parse(html.match(/<script id="viewer-generation" type="application\/json">(.*?)<\/script>/s)[1]).generator.revision;
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const files = ['planorama.js', 'planorama_bg.wasm'];
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--format-version', '1', '--no-default-features', '--filter-platform', 'wasm32-unknown-unknown'], {encoding: 'utf8', maxBuffer: 32 * 1024 * 1024}));
require('../.github/scripts/release.cjs').notices(metadata.packages, directory);
for (const file of ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE']) fs.copyFileSync(file, path.join(directory, file));
files.push('LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY-NOTICES.txt');
for (const file of files) {
  const bytes = fs.readFileSync(path.join(directory, file));
  if (/<symbol\b|<image\b|data:image\//i.test(bytes.toString('utf8'))) {
    throw new Error(`Artwork marker in ${file}`);
  }
}
fs.writeFileSync(path.join(directory, 'manifest.json'), JSON.stringify({
  version, commit, icon_artwork: false, wasm_bindgen: '0.2.100',
  memory: JSON.parse(fs.readFileSync(path.join(directory, 'memory-capabilities.json'), 'utf8')),
  files: Object.fromEntries(files.map(file => [file, sha256(fs.readFileSync(path.join(directory, file)))])),
}, null, 2) + '\n');
fs.mkdirSync('output/wasm-release', {recursive: true});
const archive = `planorama-${version}-wasm-no-artwork.tar.gz`;
execFileSync('tar', ['--mtime=@0', '--owner=0', '--group=0', '--numeric-owner', '--mode=0644', '-czf', path.resolve('output/wasm-release', archive), '-C', directory, ...files, 'manifest.json']);
fs.writeFileSync(`output/wasm-release/${archive}.sha256`, `${sha256(fs.readFileSync(`output/wasm-release/${archive}`))}  ${archive}\n`);
