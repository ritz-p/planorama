const {isDeepStrictEqual} = require('node:util');
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const sorted = values => [...values].sort();

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

module.exports = {changes, structural, validateBlock};
