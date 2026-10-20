const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');

const root = path.resolve(__dirname, '..');
const fixtures = path.join(root, 'tests/fixtures/compatibility');
const matrix = JSON.parse(fs.readFileSync(path.join(fixtures, 'matrix.json')));
const work = path.join(root, 'output/terraform-compatibility');
const phase = process.argv[2];
if (!['prepare', 'capture', 'check'].includes(phase)) {
  throw new Error('Usage: node scripts/terraform-compatibility.cjs prepare|capture|check');
}
function run(binary, args, cwd) {
  return execFileSync(binary, args, {
    cwd, maxBuffer: 128 * 1024 * 1024, timeout: 300000,
    env: { ...process.env, TF_IN_AUTOMATION: '1', CHECKPOINT_DISABLE: '1',
      AWS_ACCESS_KEY_ID: 'fixture-only', AWS_SECRET_ACCESS_KEY: 'fixture-only',
      AWS_EC2_METADATA_DISABLED: 'true' },
    stdio: ['ignore', 'pipe', 'pipe']
  });
}
function write(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
}
function copyInputs(source, target, version, provider) {
  fs.mkdirSync(target, { recursive: true });
  for (const item of fs.readdirSync(source, { withFileTypes: true })) {
    if (item.name.startsWith('.')) continue;
    const from = path.join(source, item.name);
    const to = path.join(target, item.name);
    if (item.isDirectory()) copyInputs(from, to, version, provider);
    else if (item.name.endsWith('.tf') || item.name === 'shared-state.json') {
      let content = fs.readFileSync(from, 'utf8');
      content = content.replace(/required_version\s*=\s*"[^"]+"/,
        `required_version = "= ${version}"`);
      if (provider) content = content.replace(/version\s*=\s*"= (5\.100\.0|6\.0\.0)"/,
        `version = "= ${provider}"`);
      fs.writeFileSync(to, content);
    }
  }
}
function schemaContract(schema) {
  const aws = schema.provider_schemas['registry.terraform.io/hashicorp/aws'];
  if (!aws) return {};
  return Object.fromEntries(['aws_vpc', 'aws_subnet', 'aws_cloudwatch_log_group', 'aws_iam_role']
    .map(type => [type, { version: aws.resource_schemas[type].version,
      region: aws.resource_schemas[type].block.attributes.region ?? null }]));
}
for (const [version, checksum] of Object.entries(matrix.terraform)) {
  const binDir = path.join(work, 'bin', version);
  const binary = path.join(binDir, 'terraform');
  if (phase === 'prepare') {
    fs.mkdirSync(binDir, { recursive: true });
    const archive = path.join(binDir, 'terraform.zip');
    if (!fs.existsSync(archive)) {
      run('curl', ['-fsSL', `https://releases.hashicorp.com/terraform/${version}/terraform_${version}_${matrix.platform}.zip`, '-o', archive]);
    }
    assert.equal(crypto.createHash('sha256').update(fs.readFileSync(archive)).digest('hex'), checksum);
    run('unzip', ['-oq', archive, '-d', binDir]);
  }
  for (const item of matrix.cases) {
    const cwd = path.join(work, version, item.name);
    const destination = path.join(fixtures, version, item.name);
    if (phase === 'prepare') {
      copyInputs(path.join(root, item.source), cwd, version, item.provider);
      const extra = path.join(fixtures, 'inputs', `${item.name === 'aws5' || item.name === 'aws6' ? 'aws' : item.name}.tf`);
      if (fs.existsSync(extra)) fs.copyFileSync(extra, path.join(cwd, 'compatibility.tf'));
      const lock = path.join(destination, '.terraform.lock.hcl');
      if (fs.existsSync(lock)) fs.copyFileSync(lock, path.join(cwd, '.terraform.lock.hcl'));
      run(binary, ['init', '-backend=false', '-input=false', ...(fs.existsSync(lock) ? ['-lockfile=readonly'] : [])], cwd);
    } else if (phase === 'capture') {
      assert.equal(fs.existsSync(path.join(cwd, 'terraform.tfstate')), false, 'capture requires empty managed state');
      assert.equal(JSON.parse(run(binary, ['version', '-json'], cwd)).terraform_version, version);
      run(binary, ['plan', '-refresh=false', '-input=false', '-out=capture.tfplan'], cwd);
      const plan = JSON.parse(run(binary, ['show', '-json', 'capture.tfplan'], cwd));
      assert.equal(plan.terraform_version, version);
      delete plan.timestamp;
      write(path.join(destination, 'plan.json'), plan);
      write(path.join(destination, 'schema.json'), schemaContract(JSON.parse(run(binary, ['providers', 'schema', '-json'], cwd))));
      if (item.provider) {
        fs.copyFileSync(path.join(cwd, '.terraform.lock.hcl'), path.join(destination, '.terraform.lock.hcl'));
        assert.match(fs.readFileSync(path.join(destination, '.terraform.lock.hcl'), 'utf8'),
          new RegExp(`version\\s*=\\s*"${item.provider.replaceAll('.', '\\.')}"`));
      }
    } else {
      const plan = JSON.parse(fs.readFileSync(path.join(destination, 'plan.json')));
      assert.equal(plan.terraform_version, version);
      assert.equal(plan.format_version, '1.2');
      assert.equal(plan.errored, false);
      assert.equal(plan.complete, true);
      assert.equal(plan.timestamp, undefined);
      if (item.provider) {
        assert.equal(plan.configuration.provider_config.aws.version_constraint, item.provider);
        assert.match(fs.readFileSync(path.join(destination, '.terraform.lock.hcl'), 'utf8'),
          new RegExp(`version\\s*=\\s*"${item.provider.replaceAll('.', '\\.')}"`));
      }
      if (item.name === 'aws5' || item.name === 'aws6') {
        assert.equal(plan.resource_changes.length, 9);
        const resources = plan.configuration.root_module.resources;
        assert.deepEqual(resources.find(resource => resource.type === 'aws_ecs_task_definition')
          .expressions.container_definitions, {
          references: ['module.network.subnet_ids[0]', 'module.network.subnet_ids', 'module.network']
        });
        assert.equal(resources.find(resource => resource.type === 'aws_security_group').expressions.ingress, undefined);
        assert.equal(plan.resource_changes.find(resource => resource.type === 'aws_security_group').change.after.ingress.length, 2);
      }
      const base = path.join(fixtures, '1.9.8', item.name);
      const schema = JSON.parse(fs.readFileSync(path.join(destination, 'schema.json')));
      assert.deepEqual(schema,
        JSON.parse(fs.readFileSync(path.join(base, 'schema.json'))));
      if (item.provider) {
        assert.equal(schema.aws_iam_role.region, null);
        for (const type of ['aws_vpc', 'aws_subnet', 'aws_cloudwatch_log_group']) {
          if (item.provider === '5.100.0') assert.equal(schema[type].region, null);
          else {
            assert.equal(schema[type].region.type, 'string');
            assert.equal(schema[type].region.optional, true);
            assert.equal(schema[type].region.computed, true);
          }
        }
      }
      const baseline = JSON.parse(fs.readFileSync(path.join(base, 'plan.json')));
      const relevance = entries => (entries ?? []).map(entry => JSON.stringify(entry)).sort();
      if (version === '1.16.6' && item.name === 'language') {
        for (const address of ['data.terraform_remote_state.after_ready', 'data.terraform_remote_state.existing']) {
          const before = baseline.relevant_attributes.find(entry => entry.resource === address);
          const after = plan.relevant_attributes.find(entry => entry.resource === address);
          assert.deepEqual(before.attribute, ['outputs', 'network_id']);
          assert.deepEqual(after.attribute, ['outputs']);
          after.attribute = before.attribute;
        }
        process.stdout.write('known difference: remote-state relevance broadens to outputs\n');
      }
      assert.deepEqual(relevance(plan.relevant_attributes), relevance(baseline.relevant_attributes));
      if (version === '1.16.6' && item.name === 'language') {
        function removeUnusedStore(module) {
          for (const resource of module.resources ?? []) {
            if (resource.type === 'terraform_data') {
              assert.equal(resource.values.store, null);
              delete resource.values.store;
            }
          }
          for (const child of module.child_modules ?? []) removeUnusedStore(child);
        }
        removeUnusedStore(plan.planned_values.root_module);
        for (const resource of plan.resource_changes) {
          if (resource.type === 'terraform_data') {
            assert.equal(resource.change.after.store, null);
            delete resource.change.after.store;
          }
        }
        process.stdout.write('known difference: terraform_data adds unused store=null\n');
      }
      if (version === '1.16.6' && item.name === 'aws6') {
        const lb = plan.planned_values.root_module.resources.find(resource => resource.address === 'aws_lb.app');
        assert.equal(lb.identity_schema_version, 0);
        assert.deepEqual(lb.identity, { arn: null });
        delete lb.identity_schema_version;
        delete lb.identity;
        const change = plan.resource_changes.find(resource => resource.address === 'aws_lb.app').change;
        assert.deepEqual(change.after_identity, { arn: null });
        delete change.after_identity;
        process.stdout.write('known difference: aws_lb.app adds unknown ARN identity metadata\n');
      }
      for (const field of ['planned_values', 'resource_changes', 'configuration', 'checks']) {
        assert.deepEqual(plan[field], baseline[field], `${version}/${item.name}: ${field}`);
      }
    }
    process.stdout.write(`${phase}: ${version}/${item.name}\n`);
  }
}
