const {readFileSync, writeFileSync} = require('node:fs');

const source = readFileSync('src/view/purpose.rs', 'utf8');
const definition = source.match(/const TYPES: \[&str; (\d+)\] = \[([\s\S]*?)\];/);
if (!definition) throw new Error('Spatial type definition not found');
const types = Array.from(definition[2].matchAll(/"(aws_[a-z0-9_]+)"/g), match => match[1]);
if (types.length !== Number(definition[1]) || new Set(types).size !== types.length) {
  throw new Error('Invalid spatial type definition');
}
const template = readFileSync('browser/index.html', 'utf8');
const marker = '<!-- spatial-type-options -->';
if (template.split(marker).length !== 2) throw new Error('Spatial options placeholder not found');
writeFileSync('output/browser/index.html', template.replace(marker, types.map(type =>
  `<label><input id="spatial-${type}" type="checkbox" value="${type}"> ${type}</label>`
).join('\n')));
