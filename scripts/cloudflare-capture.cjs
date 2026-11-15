const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {execFile} = require('node:child_process');
const {promisify} = require('node:util');
const http = require('node:http');

const root = path.resolve(__dirname, '..');
const fixtures = path.join(root, 'tests/fixtures/cloudflare-captured');
const work = path.join(root, 'output/cloudflare-captured');
const source = 'registry.terraform.io/cloudflare/cloudflare';
const version = '1.9.8';
const providerVersion = '5.28.1';
const matrix = JSON.parse(fs.readFileSync(path.join(root, 'tests/fixtures/compatibility/matrix.json')));
const binary = path.join(work, 'bin/terraform');
const cliConfig = 'disable_checkpoint = true\n';
const cases = ['create', 'update', 'replace', 'data'];
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
const fileHash = file => hash(fs.readFileSync(file));
const write = (file, value) => fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
const read = file => JSON.parse(fs.readFileSync(file));

function inputs(directory = fixtures) {
  return Object.fromEntries(['main.tf', 'data.tf', '.terraform.lock.hcl'].map(name => [name, fileHash(path.join(directory, name))]));
}

async function run(command, args, cwd = work, baseURL = 'http://127.0.0.1:1/') {
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
    !/^(TF_|CLOUDFLARE_|CLOUDFLARE$|HTTP_PROXY$|HTTPS_PROXY$|ALL_PROXY$|NO_PROXY$)/i.test(key)));
  return (await promisify(execFile)(command, args, {
    cwd, timeout: 300000, maxBuffer: 128 * 1024 * 1024,
    env: {...env, TF_IN_AUTOMATION: '1', CHECKPOINT_DISABLE: '1',
      TF_CLI_CONFIG_FILE: path.join(work, 'terraform.rc'),
      CLOUDFLARE_API_TOKEN: 'fixture-only-not-a-real-token'.padEnd(40, '0'),
      CLOUDFLARE_BASE_URL: baseURL},
  })).stdout;
}

async function prepare() {
  assert.equal(process.platform, 'linux');
  assert.equal(process.arch, 'x64');
  fs.mkdirSync(path.join(work, 'bin'), {recursive: true});
  fs.writeFileSync(path.join(work, 'terraform.rc'), cliConfig);
  const archive = path.join(work, 'bin/terraform.zip');
  if (!fs.existsSync(archive)) await run('curl', ['-fsSL', `https://releases.hashicorp.com/terraform/${version}/terraform_${version}_linux_amd64.zip`, '-o', archive]);
  assert.equal(fileHash(archive), matrix.terraform[version]);
  await run('unzip', ['-oq', archive, '-d', path.dirname(binary)]);
  for (const name of cases) {
    const cwd = path.join(work, name);
    fs.rmSync(cwd, {recursive: true, force: true});
    fs.mkdirSync(cwd);
    fs.copyFileSync(path.join(fixtures, 'main.tf'), path.join(cwd, 'main.tf'));
    if (name === 'data') fs.copyFileSync(path.join(fixtures, 'data.tf'), path.join(cwd, 'data.tf'));
    const lock = path.join(fixtures, '.terraform.lock.hcl');
    if (fs.existsSync(lock)) fs.copyFileSync(lock, path.join(cwd, '.terraform.lock.hcl'));
    await run(binary, ['init', '-backend=false', '-input=false', ...(fs.existsSync(lock) ? ['-lockfile=readonly'] : [])], cwd);
    if (!fs.existsSync(lock)) fs.copyFileSync(path.join(cwd, '.terraform.lock.hcl'), lock);
  }
  write(path.join(work, 'prepared.json'), {inputs: inputs(), binarySha256: fileHash(binary)});
}

function seed(plan, schema) {
  const ids = {
    cloudflare_zone: {id: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'},
    cloudflare_dns_record: {id: 'dddddddddddddddddddddddddddddddd'},
    cloudflare_worker: {id: '11111111-1111-4111-8111-111111111111'},
    cloudflare_workers_kv_namespace: {id: 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee'},
    cloudflare_r2_bucket: {id: 'fixture-assets'},
    cloudflare_d1_database: {id: '22222222-2222-4222-8222-222222222222', uuid: '22222222-2222-4222-8222-222222222222'},
  };
  return {version: 4, terraform_version: version, serial: 1, lineage: '33333333-3333-4333-8333-333333333333', outputs: {},
    resources: plan.planned_values.root_module.resources.filter(r => ids[r.type]).map(r => ({
      mode: r.mode, type: r.type, name: r.name, provider: `provider["${source}"]`,
      instances: [{schema_version: schema.resource_schemas[r.type].version,
        attributes: {...r.values, ...ids[r.type], ...(r.type === 'cloudflare_dns_record' ? {zone_id: ids.cloudflare_zone.id} : {})},
        sensitive_attributes: []}],
    }))};
}

function normalize(plan) {
  delete plan.timestamp;
  plan.relevant_attributes?.sort((a, b) => Buffer.compare(Buffer.from(JSON.stringify(a)), Buffer.from(JSON.stringify(b))));
  return plan;
}

function verifyPrepared(cwd, name) {
  const files = ['main.tf', '.terraform.lock.hcl', ...(name === 'data' ? ['data.tf'] : [])];
  assert.deepEqual(fs.readdirSync(cwd).sort(), ['.terraform', ...files].sort(),
    'unexpected prepared files; rerun prepare');
  for (const file of files) {
    assert.equal(fileHash(path.join(cwd, file)), inputs()[file], 'prepared input changed; rerun prepare');
  }
}

function contract(directory = fixtures) {
  return {
    kind: 'provider-generated-plan-with-artificial-state-and-loopback-data',
    terraformVersion: version, terraformArchiveSha256: matrix.terraform[version],
    provider: source, providerVersion,
    inputs: inputs(directory),
    generatorSha256: fileHash(__filename),
    plans: Object.fromEntries(cases.map(name => [name, fileHash(path.join(directory, `${name}.json`))])),
    seedSha256: fileHash(path.join(directory, 'seed.json')),
    schemaSha256: fileHash(path.join(directory, 'schema.json')),
    limits: [
      'Actual Terraform/Core and official Provider plan/show output; timestamp removed and unordered relevant_attributes sorted, without dropping entries.',
      'Update/replacement use generated artificial local state, not state from apply or a deployed Cloudflare account.',
      'Replacement is requested with -replace=cloudflare_workers_kv_namespace.cache, not caused by a schema ForceNew change.',
      'Data source reads use a loopback HTTP fixture; deferred reads are not executed.',
      'No live Cloudflare API, authentication, apply, refresh, deployment or runtime behavior is verified.',
    ],
  };
}

async function capture() {
  assert.deepEqual(read(path.join(work, 'prepared.json')), {inputs: inputs(), binarySha256: fileHash(binary)}, 'sources or binary changed; rerun prepare');
  assert.equal(fs.readFileSync(path.join(work, 'terraform.rc'), 'utf8'), cliConfig, 'prepared CLI configuration changed');
  const requests = [];
  const server = http.createServer((req, res) => {
    requests.push(`${req.method} ${req.url}`);
    if (req.method !== 'GET' || req.url !== '/zones/cccccccccccccccccccccccccccccccc') {
      res.writeHead(400).end('unexpected fixture request');
      return;
    }
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify({success: true, errors: [], messages: [], result: {
      id: 'cccccccccccccccccccccccccccccccc', name: 'lookup.invalid', type: 'full', status: 'active',
      account: {id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', name: 'Fixture account'},
    }}));
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  try {
    const baseURL = `http://127.0.0.1:${server.address().port}/`;
    let initial, schema;
    for (const name of cases) {
      const cwd = path.join(work, name);
      verifyPrepared(cwd, name);
      const tools = JSON.parse(await run(binary, ['version', '-json'], cwd));
      assert.equal(tools.terraform_version, version);
      assert.deepEqual(tools.provider_selections, {[source]: providerVersion});
      if (!schema) {
        schema = JSON.parse(await run(binary, ['providers', 'schema', '-json'], cwd)).provider_schemas[source];
        const types = ['cloudflare_zone', 'cloudflare_dns_record', 'cloudflare_worker', 'cloudflare_worker_version', 'cloudflare_workers_kv_namespace', 'cloudflare_r2_bucket', 'cloudflare_d1_database'];
        const excerpt = {
          resource_schemas: Object.fromEntries(types.map(type => [type, schema.resource_schemas[type]])),
          data_source_schemas: {cloudflare_zone: schema.data_source_schemas.cloudflare_zone},
        };
        write(path.join(fixtures, 'schema.json'), JSON.parse(JSON.stringify(excerpt,
          (key, value) => ['description', 'description_kind'].includes(key) ? undefined : value)));
      }
      if (name === 'update' || name === 'replace') {
        const state = seed(initial, schema);
        write(path.join(fixtures, 'seed.json'), state);
        write(path.join(cwd, 'terraform.tfstate'), state);
      }
      await run(binary, ['plan', '-refresh=false', '-input=false', '-out=capture.tfplan',
        ...(name === 'update' ? ['-var=revision=updated'] : []),
        ...(name === 'replace' ? ['-replace=cloudflare_workers_kv_namespace.cache'] : [])], cwd, baseURL);
      const plan = normalize(JSON.parse(await run(binary, ['show', '-json', 'capture.tfplan'], cwd, baseURL)));
      assert.equal(plan.errored, false);
      assert.equal(plan.complete, true);
      write(path.join(fixtures, `${name}.json`), plan);
      if (name === 'create') initial = plan;
      if (name !== 'data') assert.deepEqual(requests, [], 'managed planning must not call HTTP');
    }
    assert.deepEqual(requests, ['GET /zones/cccccccccccccccccccccccccccccccc']);
    write(path.join(fixtures, 'capture.json'), contract());
  } finally { await new Promise(resolve => server.close(resolve)); }
}

function check(directory = fixtures) {
  assert.deepEqual(read(path.join(directory, 'capture.json')), contract(directory), 'capture differs from sources or tools');
  assert.match(fs.readFileSync(path.join(directory, '.terraform.lock.hcl'), 'utf8'), /version\s*=\s*"5\.28\.1"/);
  for (const name of cases) {
    const plan = read(path.join(directory, `${name}.json`));
    assert.equal(plan.terraform_version, version);
    assert.equal(plan.format_version, '1.2');
    assert.deepEqual(plan, normalize(structuredClone(plan)));
    assert.equal(plan.errored, false);
    assert.equal(plan.complete, true);
    assert.equal(plan.configuration.provider_config.cloudflare.version_constraint, '5.28.1');
  }
}

if (require.main === module) {
  const actions = {prepare, capture, check};
  const action = Object.hasOwn(actions, process.argv[2]) && actions[process.argv[2]];
  if (!action || process.argv.length !== 3) throw new Error('Usage: node scripts/cloudflare-capture.cjs prepare|capture|check');
  Promise.resolve().then(action).then(() => console.log(`Cloudflare provider capture: ${process.argv[2]} passed`)).catch(error => {
    console.error(error.stderr || error);
    process.exitCode = 1;
  });
}

module.exports = {check, seed, cases, normalize, verifyPrepared};
