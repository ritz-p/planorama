const {readFileSync} = require('node:fs');
const {resolve} = require('node:path');
const root = resolve(__dirname, '../..');
const directory = process.argv[2] || 'output/wasm-engine';
const binding = new Function(readFileSync(`${directory}/planorama.js`, 'utf8') + '\nreturn wasm_bindgen;')();
binding.initSync({module: readFileSync(`${directory}/planorama_bg.wasm`), memory_budget_mib: 512});
require('./wasm-s3.cjs')({
  root,
  native: require('../support/native-reference.cjs')(resolve(root, 'target/release/planorama')),
  run: (input, options) => ({ok: true, ...JSON.parse(binding.generate(input, JSON.stringify(options)))}),
});
console.log('S3 native/Wasm SVG and HTML parity passed');
