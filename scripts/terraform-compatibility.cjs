const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync } = require('node:child_process');
const assert = require('node:assert/strict');
const { normalizeProvider } = require('./compatibility-provider.cjs');
const { readInputs, sourceInputs, digest, recreateInputs } = require('./terraform-compatibility-inputs.cjs');

const root = path.resolve(__dirname, '..');
const fixtures = path.join(root, 'tests/fixtures/compatibility');
const matrix = JSON.parse(fs.readFileSync(path.join(fixtures, 'matrix.json')));
const work = path.join(root, 'output/terraform-compatibility');
const phase = process.argv[2];
const selected = process.argv.slice(3);
for (const name of selected) assert(matrix.cases.some(item => item.name === name), `unknown case: ${name}`);
if (!['prepare', 'capture', 'check'].includes(phase)) {
  throw new Error('Usage: node scripts/terraform-compatibility.cjs prepare|capture|check [case ...]');
}
const cache = path.join(work, 'plugins');
if (phase === 'prepare') fs.mkdirSync(cache, { recursive: true });
function run(binary, args, cwd) {
  return execFileSync(binary, args, {
    cwd, maxBuffer: 128 * 1024 * 1024, timeout: 300000,
    env: { ...process.env, TF_IN_AUTOMATION: '1', CHECKPOINT_DISABLE: '1', TF_PLUGIN_CACHE_DIR: cache,
      AWS_ACCESS_KEY_ID: 'fixture-only', AWS_SECRET_ACCESS_KEY: 'fixture-only',
      AWS_EC2_METADATA_DISABLED: 'true' },
    stdio: ['ignore', 'pipe', 'pipe']
  });
}
function write(file, value) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
}
function fileHash(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}
function captureContract(destination, inputs, checksum, provider) {
  return {
    inputs_sha256: digest(inputs),
    terraform_archive_sha256: checksum,
    plan_sha256: fileHash(path.join(destination, 'plan.json')),
    schema_sha256: fileHash(path.join(destination, 'schema.json')),
    provider_lock_sha256: provider ? fileHash(path.join(destination, '.terraform.lock.hcl')) : null
  };
}
function schemaContract(schema) {
  const aws = schema.provider_schemas['registry.terraform.io/hashicorp/aws'];
  if (!aws) return {};
  return Object.fromEntries(['aws_vpc', 'aws_subnet', 'aws_cloudwatch_log_group', 'aws_iam_role']
    .map(type => [type, { version: aws.resource_schemas[type].version,
      region: aws.resource_schemas[type].block.attributes.region ?? null }]));
}
for (const [version, checksum] of Object.entries(matrix.terraform)) {
  const cases = matrix.cases.filter(item => (!selected.length || selected.includes(item.name)) && (!item.cores || item.cores.includes(version)));
  if (!cases.length) continue;
  assert.match(version, /^\d+\.\d+\.\d+$/);
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
  for (const item of cases) {
    if (item.cores && !item.cores.includes(version)) continue;
    assert.match(item.name, /^[a-z][a-z0-9-]*$/);
    const cwd = path.join(work, version, item.name);
    const destination = path.join(fixtures, version, item.name);
    const extra = path.join(fixtures, 'inputs', `${item.input || (item.name === 'aws5' || item.name === 'aws6' ? 'aws' : item.name)}.tf`);
    const inputs = sourceInputs(path.join(root, item.source), extra, version, item.provider);
    if (phase === 'prepare') {
      recreateInputs(work, cwd, inputs);
      const lock = path.join(destination, '.terraform.lock.hcl');
      if (fs.existsSync(lock)) fs.copyFileSync(lock, path.join(cwd, '.terraform.lock.hcl'));
      run(binary, ['init', '-backend=false', '-input=false', ...(fs.existsSync(lock) ? ['-lockfile=readonly'] : [])], cwd);
    } else if (phase === 'capture') {
      assert.equal(digest(readInputs(cwd)), digest(inputs), 'prepared inputs differ from current sources; rerun prepare');
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
      write(path.join(destination, 'capture.json'), captureContract(destination, inputs, checksum, item.provider));
    } else {
      assert.deepEqual(JSON.parse(fs.readFileSync(path.join(destination, 'capture.json'))),
        captureContract(destination, inputs, checksum, item.provider), `${version}/${item.name}: capture differs from sources or tools`);
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
      if (item.name === 'aws5' || item.name === 'aws6' || item.name === 'aws670') {
        assert.equal(plan.resource_changes.length, 9);
        const resources = plan.configuration.root_module.resources;
        assert.deepEqual(resources.find(resource => resource.type === 'aws_ecs_task_definition')
          .expressions.container_definitions, {
          references: ['module.network.subnet_ids[0]', 'module.network.subnet_ids', 'module.network']
        });
        assert.equal(resources.find(resource => resource.type === 'aws_security_group').expressions.ingress, undefined);
        assert.equal(plan.resource_changes.find(resource => resource.type === 'aws_security_group').change.after.ingress.length, 2);
      }
      const base = path.join(fixtures, '1.9.8', item.compare || item.name);
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
      if (item.provider === '6.70.0') normalizeProvider(plan);
      const relevance = entries => (entries ?? []).map(entry => JSON.stringify(entry)).sort();
      if (['1.13.5', '1.14.9', '1.15.9', '1.16.6'].includes(version) && item.name === 'language') {
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
