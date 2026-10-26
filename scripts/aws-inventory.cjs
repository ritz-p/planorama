const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const [source, schemaPath, output, version = '6.70.0', commit = '5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f'] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(version) || !/^[a-f0-9]{40}$/.test(commit)) throw new Error('Expected pinned version and full commit');
if (![3, 5].includes(process.argv.length - 2) || !source || !schemaPath || !output) throw new Error('Usage: node scripts/aws-inventory.cjs SOURCE SCHEMA OUTPUT [VERSION COMMIT]');
const schemaBytes = fs.readFileSync(schemaPath);
const schema = JSON.parse(schemaBytes).provider_schemas['registry.terraform.io/hashicorp/aws'];
const registrations = new Map();
const services = new Map();
const names = fs.readFileSync(path.join(source, 'names/data/names_data.hcl'), 'utf8');
for (const block of names.matchAll(/^service "([^"]+)" \{[^]*?^}/gm)) {
  const name = block[0].match(/human_friendly\s*=\s*"([^"]+)"/)?.[1];
  const brand = block[0].match(/brand\s*=\s*"([^"]+)"/)?.[1];
  const service = block[0].match(/provider_package_correct\s*=\s*"([^"]+)"/)?.[1] ?? block[1];
  if (name) {
    const label = brand ? `${brand} ${name}` : name;
    services.set(service, label);
    services.set(block[1], label);
  }
}
for (const service of fs.readdirSync(path.join(source, 'internal/service')).sort()) {
  const filename = path.join(source, 'internal/service', service, 'service_package_gen.go');
  if (!fs.existsSync(filename)) continue;
  const code = fs.readFileSync(filename, 'utf8');
  for (const section of code.matchAll(/func \(p \*servicePackage\) (?:Framework|SDK)Resources\([^]*?\n}\n/g)) {
    const entries = section[0].split(/\n\t\t\{\n/).slice(1);
    for (const entry of entries) {
      const type = entry.match(/TypeName:\s*"([^"]+)"/)?.[1];
      if (!type || registrations.has(type)) throw new Error(`Invalid registration: ${type}`);
      registrations.set(type, {service, factory: entry.match(/Factory:\s*(\w+)/)?.[1], region: entry.match(/Region:\s*inttypes\.(\w+)\(/)?.[1] ?? 'Unspecified'});
    }
  }
}
const allRegistrations = new Map(registrations);
const resources = Object.keys(schema.resource_schemas).sort().map(type => {
  const registration = registrations.get(type);
  if (!registration) throw new Error(`Missing registration: ${type}`);
  registrations.delete(type);
  let filename = `website/docs/r/${type.slice(4)}.html.markdown`;
  let aliasOf;
  if (!fs.existsSync(path.join(source, filename))) {
    aliasOf = [...allRegistrations].find(([other, entry]) => other !== type && entry.service === registration.service && entry.factory === registration.factory && fs.existsSync(path.join(source, `website/docs/r/${other.slice(4)}.html.markdown`)))?.[0];
    if (!aliasOf) throw new Error(`Missing resource docs: ${type}`);
    filename = `website/docs/r/${aliasOf.slice(4)}.html.markdown`;
  }
  const doc = fs.readFileSync(path.join(source, filename), 'utf8');
  const category = doc.match(/^subcategory:\s*"([^"]+)"/m)?.[1];
  if (!category) throw new Error(`Missing category: ${type}`);
  const block = schema.resource_schemas[type].block;
  const serviceName = services.get(registration.service);
  if (!serviceName) throw new Error(`Missing service name: ${registration.service}`);
  return {type, ...registration, category, serviceName, documentation: filename, aliasOf,
    description: doc.match(/^description: \|-\r?\n\s+([^\n]+)/m)?.[1]?.trim() ?? '',
    deprecated: Boolean(block.deprecated),
    regionAttribute: Boolean(block.attributes?.region),
    dataSource: Object.hasOwn(schema.data_source_schemas, type)};
});
if (registrations.size) throw new Error(`Missing schema: ${[...registrations.keys()]}`);
const inventory = {version, commit,
  schemaSha256: crypto.createHash('sha256').update(schemaBytes).digest('hex'), resources};
fs.writeFileSync(output, JSON.stringify(inventory, null, 2) + '\n');
console.log(`${resources.length} managed resources; ${new Set(resources.map(r => r.service)).size} services`);
