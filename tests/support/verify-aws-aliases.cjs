const fs = require('node:fs');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const path = require('node:path');

const [source, schemaPath] = process.argv.slice(2);
const inventory = require('../fixtures/aws-inventory/6.70.0.json');
const bytes = fs.readFileSync(schemaPath);
assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), inventory.schemaSha256);
const schema = JSON.parse(bytes).provider_schemas['registry.terraform.io/hashicorp/aws'];
const registration = fs.readFileSync(path.join(source, 'internal/service/elbv2/service_package_gen.go'), 'utf8');
const aliases = inventory.resources.filter(entry => entry.aliasOf);
assert.equal(aliases.length, 6);
for (const entry of aliases) {
  const alias = entry.type;
  const canonical = entry.aliasOf;
  assert.deepEqual(schema.resource_schemas[alias], schema.resource_schemas[canonical], alias);
  const factories = type => [...registration.matchAll(/Factory:\s+(\w+),\s+TypeName:\s+"([^"]+)"/g)]
    .filter(match => match[2] === type).map(match => match[1]);
  const actual = factories(alias);
  assert.ok(actual.length > 0);
  for (const factory of actual) assert.ok(factories(canonical).includes(factory), `${alias}: ${factory}`);
  const data = schema.data_source_schemas[alias];
  if (data) assert.deepEqual(data, schema.data_source_schemas[canonical], `data.${alias}`);
  const docs = fs.readFileSync(path.join(source, entry.documentation), 'utf8');
  assert.ok(docs.includes(alias) && /identical|same functionality/i.test(docs), alias);
  console.log(`${alias} -> ${canonical}: identical managed schema; data ${data ? 'identical' : 'unregistered'}; factories ${actual.join(', ')}`);
}
