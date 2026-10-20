const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const { execFileSync, spawn } = require('node:child_process');

const root = path.resolve(__dirname, '../..');
const work = path.join(root, 'output/provider-plan');
const versions = ['1.9.8', '1.16.6'];
const checksums = JSON.parse(fs.readFileSync(path.join(root, 'tests/fixtures/compatibility/matrix.json'))).terraform;
const phase = process.argv[2];
assert.ok(['prepare', 'verify'].includes(phase), 'Usage: node tests/support/provider_plan.cjs prepare|verify');
const env = { ...process.env, CHECKPOINT_DISABLE: '1', TF_IN_AUTOMATION: '1',
  AWS_ACCESS_KEY_ID: 'fixture', AWS_SECRET_ACCESS_KEY: 'fixture', AWS_EC2_METADATA_DISABLED: 'true' };
for (const key of Object.keys(env)) if (/^TF_(CLI_ARGS|LOG)/.test(key)) delete env[key];
function run(binary, args, cwd, extra = {}) {
  return execFileSync(binary, args, { cwd, env: { ...env, ...extra }, timeout: 120000,
    maxBuffer: 128 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
}
function compact(block) {
  return { attributes: Object.fromEntries(Object.entries(block.attributes).map(([k, v]) => [k, v.type])),
    blocks: Object.fromEntries(Object.entries(block.block_types || {}).map(([k, v]) => [k, compact(v.block)])) };
}
async function main() {
  let server;
  let portFile;
  try {
    if (phase === 'verify') {
      portFile = path.join(fs.mkdtempSync('/tmp/planorama-sts-'), 'port');
      server = spawn(process.execPath, [path.join(__dirname, 'sts_fixture.cjs'), portFile], { stdio: 'ignore' });
      for (let i = 0; !fs.existsSync(portFile); i++) {
        assert.ok(i < 100 && server.exitCode === null, 'STS fixture startup failed');
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    }
    for (const version of versions) {
      const cwd = path.join(work, version);
      const binDir = path.join(work, 'bin', version);
      const binary = path.join(binDir, 'terraform');
      const tempDir = path.join(cwd, 'tmp');
      const localEnv = { PATH: `${binDir}:${env.PATH}`, TMPDIR: tempDir };
      const cli = (input, saved, output, extra = {}) => run(path.join(root, 'target/debug/planorama'),
        [input, ...(saved ? ['--provider-plan', saved] : []), '-o', output], cwd, { ...localEnv, ...extra });
      const source = fs.readFileSync(path.join(root, 'tests/fixtures/provider-plan/main.tf'), 'utf8')
        .replace('= 1.9.8', `= ${version}`);
      if (phase === 'prepare') {
        fs.mkdirSync(binDir, { recursive: true });
        fs.mkdirSync(cwd, { recursive: true });
        const archive = path.join(binDir, 'terraform.zip');
        if (!fs.existsSync(archive)) {
          run('curl', ['-fsSL', `https://releases.hashicorp.com/terraform/${version}/terraform_${version}_linux_amd64.zip`, '-o', archive]);
        }
        assert.equal(crypto.createHash('sha256').update(fs.readFileSync(archive)).digest('hex'), checksums[version]);
        run('unzip', ['-oq', archive, '-d', binDir]);
        fs.writeFileSync(path.join(cwd, 'main.tf'), source);
        fs.copyFileSync(path.join(root, `tests/fixtures/compatibility/${version}/aws5/.terraform.lock.hcl`), path.join(cwd, '.terraform.lock.hcl'));
        run(binary, ['init', '-backend=false', '-input=false', '-lockfile=readonly'], cwd);
        continue;
      }
      assert.equal(JSON.parse(run(binary, ['version', '-json'], cwd)).terraform_version, version);
      fs.mkdirSync(tempDir, { recursive: true });
      const schema = JSON.parse(run(binary, ['providers', 'schema', '-json'], cwd))
        .provider_schemas['registry.terraform.io/hashicorp/aws'].provider.block;
      assert.deepEqual(compact(schema), JSON.parse(fs.readFileSync(path.join(root, 'src/plan/evidence/aws-5.100.0.json'))));
      const staticSource = source.replace('12345', fs.readFileSync(portFile, 'utf8').trim());
      const plan = (name, text) => {
        fs.writeFileSync(path.join(cwd, 'main.tf'), text);
        run(binary, ['plan', '-refresh=false', '-input=false', `-out=${name}.plan`], cwd);
        fs.writeFileSync(path.join(cwd, `${name}.json`), run(binary, ['show', '-json', `${name}.plan`], cwd));
      };
      const hasAccount = name => /data-account-panel="true"/.test(fs.readFileSync(path.join(cwd, name), 'utf8'));
      plan('static', staticSource);
      cli('static.json', 'static.plan', 'static.svg', {
        TF_CLI_ARGS: '-invalid-secret', TF_CLI_ARGS_show: '-invalid-secret',
        TF_LOG: 'TRACE', TF_LOG_PATH: path.join(cwd, 'must-not-exist.log'),
      });
      assert.ok(fs.readFileSync(path.join(cwd, 'static.svg'), 'utf8').includes('AWS Account: 123456789012'));
      assert.ok(!fs.existsSync(path.join(cwd, 'must-not-exist.log')));
      cli('static.json', null, 'unknown.svg');
      assert.ok(!hasAccount('unknown.svg'));
      run(path.join(root, 'target/debug/planorama'), ['--state', 'first=static.json', '--state', 'second=static.json',
        '--state-provider-plan', 'first=static.plan', '-o', 'states.svg'], cwd, localEnv);
      assert.equal((fs.readFileSync(path.join(cwd, 'states.svg'), 'utf8').match(/data-account-panel="true"/g) || []).length, 1);
      plan('dynamic', staticSource.replace('  assume_role {', `  dynamic "assume_role" {
    for_each = ["arn:aws:iam::210987654321:role/Dynamic"]
    content { role_arn = assume_role.value }
  }
  assume_role {`));
      cli('dynamic.json', 'dynamic.plan', 'dynamic.svg');
      assert.ok(!hasAccount('dynamic.svg'));
      assert.throws(() => run(path.join(root, 'target/debug/planorama'),
        ['--state', 'first=static.json', '--state', 'second=dynamic.json',
          '--state-provider-plan', 'second=static.plan', '-o', 'rejected-state.svg'], cwd, localEnv),
      error => error.status !== 0 && error.stderr.toString().includes('does not match'));
      plan('variable', staticSource.replace('region = "us-east-1"', 'region = var.region') +
        '\nvariable "region" { default = "us-east-1" }\n');
      cli('variable.json', 'variable.plan', 'variable.svg');
      assert.ok(!hasAccount('variable.svg'));
      for (const [json, saved, extra, message] of [
        ['static.json', 'dynamic.plan', {}, 'does not match'],
        ['static.json', 'static.plan', { PATH: `${path.join(work, 'bin', versions.find(v => v !== version))}:${env.PATH}` }, 'binary version'],
      ]) {
        assert.throws(() => cli(json, saved, 'rejected.svg', extra), error =>
          error.status !== 0 && error.stderr.toString().includes(message) && !error.stderr.toString().includes('arn:aws:'));
      }
      const otherBinary = path.join(work, 'bin', versions.find(v => v !== version), 'terraform');
      assert.throws(() => run(otherBinary, ['show', '-json', 'static.plan'], cwd));
      assert.deepEqual(fs.readdirSync(tempDir), []);
      console.log(`Verified saved-plan evidence: Terraform ${version} / AWS 5.100.0`);
    }
  } finally {
    if (server) {
      server.kill();
      await new Promise(resolve => server.once('exit', resolve));
    }
    if (portFile) fs.rmSync(path.dirname(portFile), { recursive: true, force: true });
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
