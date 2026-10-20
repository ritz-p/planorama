const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');

function readInputs(directory, prefix = '') {
  const files = new Map();
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.name.startsWith('.')) continue;
    const absolute = path.join(directory, entry.name);
    const relative = `${prefix}${entry.name}`;
    if (entry.isDirectory()) {
      for (const [name, content] of readInputs(absolute, `${relative}/`)) files.set(name, content);
    } else if (entry.name.endsWith('.tf') || entry.name === 'shared-state.json') {
      assert(!entry.isSymbolicLink(), `input cannot be a symlink: ${relative}`);
      files.set(relative, fs.readFileSync(absolute, 'utf8').replaceAll('\r\n', '\n'));
    }
  }
  return files;
}

function sourceInputs(source, extra, version, provider) {
  const files = readInputs(source);
  for (const [name, content] of files) {
    let prepared = content.replace(/required_version\s*=\s*"[^"]+"/,
      `required_version = "= ${version}"`);
    if (provider) prepared = prepared.replace(/version\s*=\s*"= (5\.100\.0|6\.0\.0)"/,
      `version = "= ${provider}"`);
    files.set(name, prepared);
  }
  if (fs.existsSync(extra)) {
    assert(!files.has('compatibility.tf'), 'source conflicts with compatibility.tf');
    files.set('compatibility.tf', fs.readFileSync(extra, 'utf8').replaceAll('\r\n', '\n'));
  }
  return files;
}

function digest(files) {
  return crypto.createHash('sha256')
    .update(JSON.stringify([...files].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)))
    .digest('hex');
}

function recreateInputs(workspace, directory, files) {
  const base = path.resolve(workspace);
  const target = path.resolve(directory);
  const relative = path.relative(base, target);
  assert(relative && !relative.startsWith('..') && !path.isAbsolute(relative),
    'case directory must be inside the compatibility workspace');
  let parent = base;
  for (const part of relative.split(path.sep)) {
    assert(!fs.existsSync(parent) || !fs.lstatSync(parent).isSymbolicLink(),
      'case parent cannot be a symlink');
    parent = path.join(parent, part);
  }
  fs.rmSync(target, { recursive: true, force: true });
  fs.mkdirSync(target, { recursive: true });
  for (const [name, content] of files) {
    const output = path.join(target, name);
    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.writeFileSync(output, content);
  }
}

module.exports = { readInputs, sourceInputs, digest, recreateInputs };
