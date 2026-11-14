const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const [source, schemaPath, output] = process.argv.slice(2);
if (!source || !schemaPath || !output || process.argv.length !== 5) throw new Error('Usage: node scripts/cloudflare-inventory.cjs SOURCE SCHEMA OUTPUT');
const bytes = fs.readFileSync(schemaPath);
const schema = JSON.parse(bytes).provider_schemas['registry.terraform.io/cloudflare/cloudflare'];
const provider = fs.readFileSync(path.join(source, 'internal/provider.go'), 'utf8');
function shape(a) {
  const result = {};
  for (const key of ['type', 'required', 'optional', 'computed', 'sensitive', 'deprecated']) if (a[key] !== undefined) result[key] = a[key];
  if (a.nested_type) result.nested_type = {nesting_mode: a.nested_type.nesting_mode, attributes: attributes(a.nested_type.attributes)};
  return result;
}
function attributes(a = {}) { return Object.fromEntries(Object.entries(a).sort(([a], [b]) => a.localeCompare(b, 'en')).map(([k, v]) => [k, shape(v)])); }
function entries(mode, schemas, section) {
  const registrations = provider.match(new RegExp(`func \\(p \\*CloudflareProvider\\) ${section}\\([^]*?\\n}`))?.[0];
  if (!registrations) throw new Error(`Missing ${section}`);
  const registered = new Map();
  for (const [, service, factory] of registrations.matchAll(/\b(\w+)\.(New\w+),/g)) {
    const directory = path.join(source, 'internal/services', service);
    const files = fs.readdirSync(directory).filter(f => f.endsWith('.go') && !f.endsWith('_test.go'));
    const file = files.find(f => fs.readFileSync(path.join(directory, f), 'utf8').includes(`func ${factory}(`));
    if (!file) throw new Error(`Missing factory ${service}.${factory}`);
    const code = fs.readFileSync(path.join(directory, file), 'utf8');
    const suffix = code.match(/resp.TypeName = req.ProviderTypeName \+ "([^"]+)"/)?.[1];
    const type = `cloudflare${suffix}`;
    if (!suffix || registered.has(type)) throw new Error(`Invalid registration ${type}`);
    registered.set(type, `internal/services/${service}/${file}`);
  }
  const result = Object.keys(schemas).sort().map(type => {
    const source = registered.get(type);
    if (!source) throw new Error(`Missing registration ${mode}/${type}`);
    registered.delete(type);
    const documentation = `docs/${mode === 'managed' ? 'resources' : 'data-sources'}/${type.slice(11)}.md`;
    if (!fs.existsSync(path.join(globalSource, documentation))) throw new Error(`Missing docs ${documentation}`);
    const block = schemas[type].block;
    if (Object.keys(block.block_types || {}).length) throw new Error(`Unexpected nested blocks ${type}`);
    return {type, mode, source, documentation, attributes: attributes(block.attributes)};
  });
  if (registered.size) throw new Error(`Missing schema ${[...registered.keys()]}`);
  return result;
}
const globalSource = source;
const resources = [...entries('managed', schema.resource_schemas, 'Resources'), ...entries('data', schema.data_source_schemas, 'DataSources')];
fs.writeFileSync(output, JSON.stringify({provider: 'registry.terraform.io/cloudflare/cloudflare', version: '5.28.1', commit: '9b51a582458f5c5d46d8547ed2604fcba91fb861', terraformVersion: '1.9.8', schemaSha256: crypto.createHash('sha256').update(bytes).digest('hex'), resources}, null, 2) + '\n');
console.log(`${resources.filter(r => r.mode === 'managed').length} managed; ${resources.filter(r => r.mode === 'data').length} data`);
