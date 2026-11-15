const assert = require('node:assert/strict');
const {test} = require('node:test');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
test('inventory distinguishes managed/data contracts and rejects missing registrations', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cloudflare-inventory-test-'));
  try {
    for (const p of ['internal/services/example', 'docs/resources', 'docs/data-sources']) fs.mkdirSync(path.join(dir, p), {recursive: true});
    fs.writeFileSync(path.join(dir, 'internal/provider.go'), 'func (p *CloudflareProvider) Resources(ctx context.Context) []func() resource.Resource {\n example.NewResource,\n}\nfunc (p *CloudflareProvider) DataSources(ctx context.Context) []func() datasource.DataSource {\n example.NewExampleDataSource,\n}\n');
    for (const [file, factory] of [['resource.go', 'NewResource'], ['data_source.go', 'NewExampleDataSource']]) fs.writeFileSync(path.join(dir, 'internal/services/example', file), `func ${factory}() { }\nresp.TypeName = req.ProviderTypeName + "_example"\n`);
    for (const mode of ['resources', 'data-sources']) fs.writeFileSync(path.join(dir, 'docs', mode, 'example.md'), '# Example');
    const schema = {provider_schemas: {'registry.terraform.io/cloudflare/cloudflare': {resource_schemas: {cloudflare_example: {block: {attributes: {account: {required: true, nested_type: {nesting_mode: 'single', attributes: {id: {type: 'string', optional: true}}}}}}}}, data_source_schemas: {cloudflare_example: {block: {attributes: {account_id: {type: 'string', required: true}}}}}}}};
    const schemaPath = path.join(dir, 'schema.json');
    const output = path.join(dir, 'inventory.json');
    const run = () => spawnSync(process.execPath, ['scripts/cloudflare-inventory.cjs', dir, schemaPath, output], {encoding: 'utf8'});
    fs.writeFileSync(schemaPath, JSON.stringify(schema));
    assert.equal(run().status, 0);
    const result = JSON.parse(fs.readFileSync(output));
    assert.deepEqual(result.resources.map(r => r.mode), ['managed', 'data']);
    assert.equal(result.resources[0].attributes.account.nested_type.nesting_mode, 'single');
    assert.equal(result.resources[1].attributes.account_id.required, true);
    delete schema.provider_schemas['registry.terraform.io/cloudflare/cloudflare'].data_source_schemas.cloudflare_example;
    fs.writeFileSync(schemaPath, JSON.stringify(schema));
    assert.match(run().stderr, /Missing schema cloudflare_example/);
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});
