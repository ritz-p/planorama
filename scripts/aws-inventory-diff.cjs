const fs = require('node:fs');
const crypto = require('node:crypto');
const {isDeepStrictEqual} = require('node:util');

const provider = 'registry.terraform.io/hashicorp/aws';
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const sorted = values => [...values].sort();
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

function inventory(value) {
  if (!object(value) || !/^\d+\.\d+\.\d+$/.test(value.version) || !/^[a-f0-9]{40}$/.test(value.commit)
      || !/^[a-f0-9]{64}$/.test(value.schemaSha256) || !Array.isArray(value.resources)) {
    throw new Error('Incomplete inventory: pinned version, commit, schemaSha256 and resources required');
  }
  if (value.formatVersion !== undefined && value.formatVersion !== 1) throw new Error('Unsupported inventory formatVersion');
  const entries = new Map();
  for (const entry of value.resources) {
    if (!object(entry) || !/^aws_\w+$/.test(entry.type) || entries.has(entry.type)
        || ['service', 'factory', 'region', 'category', 'serviceName', 'documentation'].some(key => typeof entry[key] !== 'string' || !entry[key])
        || ['deprecated', 'regionAttribute', 'dataSource'].some(key => typeof entry[key] !== 'boolean')) {
      throw new Error('Incomplete or duplicate inventory resource');
    }
    entries.set(entry.type, entry);
  }
  for (const entry of entries.values()) {
    if (entry.aliasOf !== undefined && (!entries.has(entry.aliasOf) || entry.aliasOf === entry.type)) throw new Error(`Invalid alias: ${entry.type}`);
  }
  return entries;
}

function schema(bytes, baseline, entries) {
  if (bytes === undefined) return undefined;
  if (hash(bytes) !== baseline.schemaSha256) throw new Error(`Schema hash mismatch for ${baseline.version}`);
  const document = JSON.parse(bytes);
  if (document.format_version !== '1.0') throw new Error(`Unsupported schema format: ${document.format_version}`);
  const value = document.provider_schemas?.[provider];
  if (!object(value?.resource_schemas) || !object(value?.data_source_schemas)) throw new Error('Incomplete AWS provider schema');
  if (!isDeepStrictEqual(sorted(Object.keys(value.resource_schemas)), sorted(entries.keys()))) throw new Error('Inventory/schema managed types mismatch');
  for (const [type, entry] of entries) {
    if (entry.deprecated !== Boolean(value.resource_schemas[type].block?.deprecated)
        || entry.dataSource !== Object.hasOwn(value.data_source_schemas, type)
        || entry.regionAttribute !== Boolean(value.resource_schemas[type].block?.attributes?.region)) throw new Error(`Inventory/schema metadata mismatch: ${type}`);
  }
  for (const section of Object.keys(value).filter(key => key.endsWith('_schemas'))) {
    if (!object(value[section])) throw new Error(`Incomplete schema section: ${section}`);
    for (const [type, resource] of Object.entries(value[section])) {
      if (!object(resource) || !object(resource.block) || !Number.isInteger(resource.version)) throw new Error(`Incomplete schema: ${section}/${type}`);
      validateBlock(resource.block, `${section}/${type}`);
    }
  }
  return value;
}

function validateBlock(block, location) {
  if (!object(block)) throw new Error(`Incomplete block: ${location}`);
  for (const field of ['attributes', 'block_types']) {
    if (block[field] !== undefined && !object(block[field])) throw new Error(`Invalid ${location}/${field}`);
  }
  for (const [name, attribute] of Object.entries(block.attributes ?? {})) {
    if (!object(attribute) || (attribute.type === undefined && !object(attribute.nested_type))) throw new Error(`Incomplete attribute: ${location}/${name}`);
    if (attribute.nested_type) {
      if (typeof attribute.nested_type.nesting_mode !== 'string') throw new Error(`Missing nesting mode: ${location}/${name}`);
      validateBlock(attribute.nested_type, `${location}/${name}`);
    }
  }
  for (const [name, nested] of Object.entries(block.block_types ?? {})) {
    if (!object(nested) || typeof nested.nesting_mode !== 'string') throw new Error(`Missing nesting mode: ${location}/${name}`);
    validateBlock(nested.block, `${location}/${name}`);
  }
}

function changes(before, after, path = '') {
  if (isDeepStrictEqual(before, after)) return [];
  if (object(before) && object(after)) return sorted(new Set([...Object.keys(before), ...Object.keys(after)]))
    .flatMap(key => changes(before[key], after[key], `${path}/${key.replace(/~/g, '~0').replace(/\//g, '~1')}`));
  return [{path: path || '/', kind: before === undefined ? 'added' : after === undefined ? 'removed' : 'changed',
    ...(before === undefined ? {} : {before}), ...(after === undefined ? {} : {after})}];
}

function structural(value, context = 'schema') {
  if (Array.isArray(value)) return value.map(entry => structural(entry, context));
  if (!object(value)) return value;
  return Object.fromEntries(sorted(Object.keys(value)).filter(key => context !== 'schema' || !['description', 'description_kind'].includes(key))
    .map(key => [key, structural(value[key], context === 'raw' || (context === 'schema' && key === 'type') ? 'raw' : ['attributes', 'block_types'].includes(key) && context === 'schema' ? 'names' : 'schema')]));
}

function capabilities(markdown) {
  const result = new Map();
  for (const line of markdown.split(/\r?\n/)) {
    const cells = line.split('|').slice(1, -1).map(cell => cell.trim());
    const type = cells[0]?.match(/^`(aws_\w+)`$/)?.[1];
    if (!type) continue;
    if (cells.length !== 7 || result.has(type)) throw new Error('Unsupported support matrix format');
    result.set(type, {role: cells[1], display: cells[2], service: cells[3], scope: cells[4], notes: cells[5], rules: cells[6]});
  }
  if (!result.size) throw new Error('Missing AWS runtime support matrix rows');
  return result;
}

function compare(oldInventory, newInventory, oldSchema, newSchema, matrix) {
  const oldEntries = inventory(oldInventory);
  const newEntries = inventory(newInventory);
  const before = schema(oldSchema, oldInventory, oldEntries);
  const after = schema(newSchema, newInventory, newEntries);
  const runtime = capabilities(matrix);
  const managed = {added: [], removed: [], changed: []};
  const review = [];
  const schemaCompared = before !== undefined && after !== undefined;
  for (const type of sorted(new Set([...oldEntries.keys(), ...newEntries.keys()]))) {
    const old = oldEntries.get(type);
    const next = newEntries.get(type);
    let reason;
    if (!old) { managed.added.push(type); reason = 'Added type: classification decision required'; }
    else if (!next) { managed.removed.push(type); reason = 'Removed type: retain historical runtime support'; }
    else {
      const metadata = changes(old, next);
      const structure = schemaCompared ? changes(structural(before.resource_schemas[type]), structural(after.resource_schemas[type])) : [];
      if (metadata.length || structure.length) {
        managed.changed.push({type, metadata, schema: schemaCompared ? {status: 'compared', changes: structure} : {status: 'not-compared'}});
        reason = 'Changed type: review classification, scope, icon and existing conditional rules';
      }
    }
    if (reason) review.push({type, reason, runtime: runtime.get(type) ?? null});
  }
  const otherSchemas = {};
  if (schemaCompared) {
    for (const section of sorted(new Set([...Object.keys(before), ...Object.keys(after)]))) {
      if (section === 'resource_schemas' || !section.endsWith('_schemas')) continue;
      otherSchemas[section] = changes(structural(before[section]), structural(after[section]));
    }
  }
  return {formatVersion: 1,
    before: {version: oldInventory.version, commit: oldInventory.commit, schemaSha256: oldInventory.schemaSha256},
    after: {version: newInventory.version, commit: newInventory.commit, schemaSha256: newInventory.schemaSha256},
    supportMatrixSha256: hash(matrix),
    schema: {status: schemaCompared ? 'compared' : 'not-compared', missing: [...(!before ? ['before'] : []), ...(!after ? ['after'] : [])]},
    managed: {beforeCount: oldEntries.size, afterCount: newEntries.size, ...managed}, otherSchemas, review,
    limitations: ['Schema differences do not prove behavior compatibility or relationship correctness.',
      'Names do not prove renames. Alias metadata requires source/documentation review.',
      'Runtime capabilities are the supplied support matrix snapshot; no implementation or baseline is changed.']};
}

if (require.main === module) {
  try {
    const [oldPath, newPath, oldSchemaPath, newSchemaPath, matrixPath, ...extra] = process.argv.slice(2);
    if (!matrixPath || extra.length) throw new Error('Usage: node scripts/aws-inventory-diff.cjs OLD_INVENTORY NEW_INVENTORY OLD_SCHEMA|- NEW_SCHEMA|- SUPPORT_MATRIX');
    const readSchema = filename => filename === '-' ? undefined : fs.readFileSync(filename);
    const report = compare(JSON.parse(fs.readFileSync(oldPath)), JSON.parse(fs.readFileSync(newPath)), readSchema(oldSchemaPath), readSchema(newSchemaPath), fs.readFileSync(matrixPath, 'utf8'));
    process.stdout.write(JSON.stringify(report, null, 2) + '\n');
  } catch (error) {
    process.stderr.write(`AWS inventory diff: ${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {compare};
