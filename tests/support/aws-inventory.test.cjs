const assert = require('node:assert/strict');
const {test} = require('node:test');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

test('AWS artwork is absent from build inputs', () => {
  assert.equal(fs.existsSync(path.resolve(__dirname, '../../assets/icons/aws')), false);
  assert.doesNotMatch(fs.readFileSync(path.resolve(__dirname, '../../build.rs'), 'utf8'), /include_str|assets/);
});

test('inventory uses managed schema and registrations, handles aliases and rejects silent omissions', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-inventory-'));
  try {
    for (const dir of ['names/data', 'internal/service/example', 'website/docs/r']) fs.mkdirSync(path.join(root, dir), {recursive:true});
    fs.writeFileSync(path.join(root, 'names/data/names_data.hcl'), 'service "example" {\n names { human_friendly = "Example" }\n brand = "AWS"\n}\n');
    const resource = type => `\n\t\t{\n\t\t\tFactory: resourceExample,\n\t\t\tTypeName: "${type}",\n\t\t\tRegion: inttypes.ResourceRegionDefault(),\n\t\t},`;
    const method = (name, types) => `func (p *servicePackage) ${name}(ctx context.Context) []Resource {\n return []Resource{${types.map(resource).join('')}\n }\n}\n`;
    fs.writeFileSync(path.join(root, 'internal/service/example/service_package_gen.go'), method('SDKResources', ['aws_example','aws_example_old']) + method('SDKDataSources', ['aws_example_data']) + method('FrameworkListResources', ['aws_example_list']) + method('FrameworkActions', ['aws_example_action']));
    fs.writeFileSync(path.join(root, 'website/docs/r/example.html.markdown'), '---\nsubcategory: "Example"\ndescription: |-\n  An example resource\n---\n');
    const schema = {provider_schemas:{'registry.terraform.io/hashicorp/aws':{resource_schemas:{aws_example:{block:{attributes:{region:{}}}},aws_example_old:{block:{deprecated:true}}},data_source_schemas:{aws_example:{}}}}};
    const schemaPath = path.join(root, 'schema.json');
    const output = path.join(root, 'inventory.json');
    const run = () => spawnSync(process.execPath, [path.resolve(__dirname, '../../scripts/aws-inventory.cjs'), root, schemaPath, output], {encoding:'utf8'});
    fs.writeFileSync(schemaPath, JSON.stringify(schema));
    assert.equal(run().status, 0);
    const result = JSON.parse(fs.readFileSync(output));
    assert.deepEqual(result.resources.map(r => r.type), ['aws_example','aws_example_old']);
    assert.equal(result.resources[0].dataSource, true);
    assert.equal(result.resources[0].serviceName, 'AWS Example');
    assert.equal(result.resources[1].aliasOf, 'aws_example');
    assert.equal(result.resources[1].deprecated, true);
    const explicit = spawnSync(process.execPath, [path.resolve(__dirname, '../../scripts/aws-inventory.cjs'), root, schemaPath, output, '7.0.0', 'b'.repeat(40)], {encoding:'utf8'});
    assert.equal(explicit.status, 0, explicit.stderr);
    const candidate = JSON.parse(fs.readFileSync(output));
    assert.equal(candidate.version, '7.0.0');
    assert.equal(candidate.commit, 'b'.repeat(40));
    assert.deepEqual(candidate.resources, result.resources);
    const floating = spawnSync(process.execPath, [path.resolve(__dirname, '../../scripts/aws-inventory.cjs'), root, schemaPath, output, 'latest', 'main'], {encoding:'utf8'});
    assert.notEqual(floating.status, 0);
    delete schema.provider_schemas['registry.terraform.io/hashicorp/aws'].resource_schemas.aws_example_old;
    fs.writeFileSync(schemaPath, JSON.stringify(schema));
    const missing = run();
    assert.notEqual(missing.status, 0);
    assert.match(missing.stderr, /Missing schema: aws_example_old/);
  } finally {
    fs.rmSync(root, {recursive:true, force:true});
  }
});
