const fs = require('node:fs');
const {createHash} = require('node:crypto');
const {isDeepStrictEqual} = require('node:util');
const {changes, structural, validateBlock} = require('./provider-schema-diff.cjs');
const {scopePaths} = require('./update-cloudflare-types.cjs');

const provider = 'registry.terraform.io/cloudflare/cloudflare';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function schema(bytes) {
  const document = JSON.parse(bytes);
  const cloudflare = document.provider_schemas?.[provider];
  if (document.format_version !== '1.0' || !cloudflare) throw new Error('Expected Terraform Cloudflare provider schema format 1.0');
  const types = new Map();
  for (const [mode, section] of [['managed', 'resource_schemas'], ['data', 'data_source_schemas']]) {
    const entries = cloudflare[section];
    if (!entries || Array.isArray(entries) || typeof entries !== 'object') throw new Error('Missing schema section ' + section);
    for (const [type, resource] of Object.entries(entries)) {
      const key = mode + '/' + type;
      if (!/^cloudflare_\w+$/.test(type) || !Number.isInteger(resource?.version)) throw new Error('Invalid resource schema ' + key);
      validateBlock(resource.block, key);
      types.set(key, resource);
    }
  }
  return types;
}

function capabilities(markdown) {
  const rows = new Map();
  for (const line of markdown.split(/\r?\n/)) {
    if (!line.startsWith('| `cloudflare_')) continue;
    const cells = line.split('|').slice(1, -1).map(s => s.trim());
    const [name, mode, level, role, display, service, references, endpoint] = cells;
    if (cells.length !== 8 || !/^`cloudflare_\w+`$/.test(name) || !['managed','data'].includes(mode)) {
      throw new Error('Invalid Cloudflare support matrix row');
    }
    const key = mode + '/' + name.slice(1, -1);
    if (rows.has(key)) throw new Error('Duplicate support matrix row ' + key);
    rows.set(key, {level, role, display, service, references, endpoint});
  }
  return rows;
}

function review(before, after, scope, delta, runtime) {
  if (!before) return ['classify-added-type'];
  if (!after) {
    const reasons = ['removed-type'];
    if (runtime.references !== '—') reasons.push('configured-references');
    if (runtime.endpoint !== '—') reasons.push('endpoint-identity');
    return reasons;
  }
  const reasons = new Set();
  const oldAttributes = before.block.attributes ?? {};
  const newAttributes = after.block.attributes ?? {};
  const oldScope = scope.before, newScope = scope.after;
  const references = [...runtime.references.matchAll(/`([^`]+)`:/g)].map(match => match[1].split('.')[0]);
  if (references.some(field => !isDeepStrictEqual(structural(oldAttributes[field]), structural(newAttributes[field])))) {
    reasons.add('configured-references');
  }
  const scopeRoots = [oldScope.account[0], oldScope.zone[0], newScope.account[0], newScope.zone[0]].filter(Boolean);
  if (!isDeepStrictEqual(oldScope, newScope)) reasons.add('scope-path');
  for (const change of delta) {
    const root = change.path.match(/^\/block\/attributes\/([^/]+)/)?.[1];
    if (scopeRoots.includes(root)) reasons.add('scope-path');
    if (/\/(optional|required|computed)$/.test(change.path)) reasons.add('input-output-contract');
    if (change.kind !== 'added' || change.after?.required === true || change.after?.min_items > 0 ||
        /\/(required|sensitive|write_only|min_items)$/.test(change.path) && Boolean(change.after)) {
      reasons.add('potentially-breaking-shape');
    }
  }
  if (runtime.endpoint !== '—') reasons.add('endpoint-identity');
  return [...reasons].sort();
}

function compare(inventory, pinnedBytes, candidateBytes, selection, markdown) {
  if (inventory.provider !== provider || inventory.schemaSha256 !== sha256(pinnedBytes)) {
    throw new Error('Pinned schema does not match the Cloudflare inventory fingerprint');
  }
  const version = selection.provider_selections?.[provider];
  if (typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(version) ||
      selection.terraform_version !== inventory.terraformVersion) throw new Error('Invalid candidate provider selection or Terraform version');
  const before = schema(pinnedBytes);
  const after = schema(candidateBytes);
  const matrix = capabilities(markdown);
  const keys = inventory.resources.map(r => r.mode + '/' + r.type).sort();
  for (const [name, entries] of [['schema', before], ['support matrix', matrix]]) {
    if (!isDeepStrictEqual(keys, [...entries.keys()].sort())) throw new Error('Pinned inventory/' + name + ' types mismatch');
  }
  const records = [];
  const counts = {managed: {added: 0, removed: 0, changed: 0}, data: {added: 0, removed: 0, changed: 0}};
  for (const key of [...new Set([...before.keys(), ...after.keys()])].sort()) {
    const [mode, type] = key.split('/');
    const old = before.get(key), next = after.get(key);
    const delta = changes(structural(old), structural(next));
    if (!delta.length) continue;
    const kind = !old ? 'added' : !next ? 'removed' : 'changed';
    const runtime = matrix.get(key) ?? null;
    counts[mode][kind]++;
    const scope = {
      before: old ? scopePaths(type, mode, old.block.attributes ?? {}) : null,
      after: next ? scopePaths(type, mode, next.block.attributes ?? {}) : null,
    };
    records.push({mode, type, kind, review: review(old, next, scope, delta, runtime), runtime, scope, changes: delta});
  }
  return {
    formatVersion: 1, provider, terraformVersion: inventory.terraformVersion,
    pinned: {version: inventory.version, commit: inventory.commit, schemaSha256: inventory.schemaSha256},
    candidate: {version, schemaSha256: sha256(candidateBytes)}, supportMatrixSha256: sha256(markdown),
    counts, records,
  };
}

if (require.main === module) {
  try {
    const [pinned, candidate, selection, matrix, ...rest] = process.argv.slice(2);
    if (!matrix || rest.length) throw new Error('Expected PINNED_SCHEMA CANDIDATE_SCHEMA CANDIDATE_TERRAFORM_VERSION_JSON SUPPORT_MATRIX');
    const inventory = require('../tests/fixtures/cloudflare-inventory/5.28.1.json');
    const report = compare(inventory, fs.readFileSync(pinned), fs.readFileSync(candidate),
      JSON.parse(fs.readFileSync(selection)), fs.readFileSync(matrix, 'utf8'));
    process.stdout.write(JSON.stringify(report, null, 2) + '\n');
  } catch (error) {
    process.stderr.write('Cloudflare inventory diff: ' + error.message + '\n');
    process.exitCode = 1;
  }
}

module.exports = {compare};
