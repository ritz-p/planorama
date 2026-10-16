const { test } = require('node:test');
const assert = require('node:assert/strict');
const { mkdtempSync, rmSync, readFileSync, existsSync, writeFileSync, mkdirSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const { spawn, execFileSync, spawnSync } = require('node:child_process');
const { once } = require('node:events');
const { parseArgs, measure } = require('./compare_test_runners.cjs');

test('comparison options retain defaults and reject invalid repetitions/concurrency', () => {
  const required = ['--suite', 'standard', '--nextest-bin', '/tmp/nextest', '--output', '/tmp/results'];
  assert.deepEqual(parseArgs(required).threads, [2, 8]);
  assert.equal(parseArgs(required).repeats, 2);
  for (const options of [['--threads'], ['--threads', '0'], ['--threads', '1.5'], ['--repeats', '1'], ['--repeats', 'NaN'], ['--output']]) {
    assert.throws(() => parseArgs([...required, ...options]));
  }
});

test('measurement records child CPU, failure status, stdout and stderr and removes timing file', () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-timing-'));
  try {
    const log = join(directory, 'command.log');
    const timing = join(directory, 'timing');
    const row = measure([process.execPath, '-e', `
      require('node:child_process').execFileSync(process.execPath, ['-e', 'const end=performance.now()+150; while(performance.now()<end) {}']);
      console.log('stdout marker'); console.error('stderr marker'); process.exitCode=7;
    `], log, timing);
    assert.equal(row.returncode, 7);
    assert.ok(row.wall_seconds > 0);
    assert.ok(row.cpu_seconds > 0.05);
    assert.ok(row.average_cpu_cores > 0);
    assert.match(readFileSync(log, 'utf8'), /stdout marker\nstderr marker/);
    assert.equal(existsSync(timing), false);
    const signal = measure([process.execPath, '-e', 'process.kill(process.pid, "SIGTERM")'], log, timing);
    assert.equal(signal.returncode, -15);
    assert.equal(existsSync(timing), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test('STS fixture binds loopback, consumes POST bodies, serves XML and terminates', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-sts-'));
  const portFile = join(directory, 'port');
  const child = spawn(process.execPath, [join(__dirname, 'sts_fixture.cjs'), portFile]);
  const exited = once(child, 'exit');
  let stderr = '';
  child.stderr.on('data', chunk => { stderr += chunk; });
  try {
    const deadline = performance.now() + 10000;
    while (!existsSync(portFile)) {
      assert.equal(child.exitCode, null, stderr);
      assert.ok(performance.now() < deadline, 'fixture startup timeout');
      await new Promise(resolve => setTimeout(resolve, 20));
    }
    const url = 'http://127.0.0.1:' + readFileSync(portFile, 'utf8');
    const response = await fetch(url, { method: 'POST', body: 'Action=AssumeRole&Version=2011-06-15', signal: AbortSignal.timeout(5000) });
    const body = await response.text();
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('content-type'), 'text/xml');
    assert.equal(Number(response.headers.get('content-length')), Buffer.byteLength(body));
    assert.match(body, /<Arn>arn:aws:sts::123456789012:assumed-role\/Fixture\/test<\/Arn>/);
    assert.match(body, /<Expiration>2099-01-01T00:00:00Z<\/Expiration>/);
    assert.equal((await fetch(url, { signal: AbortSignal.timeout(5000) })).status, 501);
  } finally {
    child.kill();
    const timer = setTimeout(() => child.kill('SIGKILL'), 2000);
    try { await exited; } finally { clearTimeout(timer); rmSync(directory, { recursive: true, force: true }); }
  }
  assert.equal(child.exitCode, 0, stderr);
});

test('comparison preserves real Cargo/nextest inventory, alternation, output and failed-run evidence', { skip: !process.env.NEXTEST_BIN }, () => {
  const directory = mkdtempSync(join(tmpdir(), 'planorama-runner-probe-'));
  const env = { ...process.env, CARGO_TARGET_DIR: join(directory, 'target') };
  const run = (command, args) => execFileSync(command, args, { cwd: directory, env, stdio: 'pipe' });
  const script = join(__dirname, 'compare_test_runners.cjs');
  try {
    mkdirSync(join(directory, 'tests'));
    writeFileSync(join(directory, 'Cargo.toml'), '[package]\nname="planorama"\nversion="0.0.0"\nedition="2021"\n');
    const cases = '#[test]\nfn ordinary() { println!("probe stdout"); eprintln!("probe stderr"); assert!(std::env::var_os("PROBE_FAIL").is_none()); }\n#[test]\n#[ignore]\nfn ignored() {}\n';
    for (const name of ['ordinary', 'large_example']) writeFileSync(join(directory, 'tests', name + '.rs'), cases);
    run('cargo', ['generate-lockfile', '--offline']);
    run('git', ['init', '-q']);
    run('git', ['add', 'Cargo.toml', 'Cargo.lock', 'tests']);
    run('git', ['-c', 'user.name=Tooling test', '-c', 'user.email=tooling@example.invalid', 'commit', '-qm', 'probe']);
    for (const suite of ['standard', 'large']) {
      const output = join(directory, suite);
      const args = [script, '--suite', suite, '--threads', '1', '2', '--repeats', '2', '--nextest-bin', process.env.NEXTEST_BIN, '--output', output];
      run(process.execPath, args);
      const report = JSON.parse(readFileSync(join(output, 'results.json'), 'utf8'));
      assert.deepEqual(Object.values(report.inventory), [{ tests: ['ordinary'], ignored: ['ignored'] }]);
      assert.deepEqual(report.runs.map(row => [row.runner, row.threads, row.repeat]), [
        ['cargo', 1, 1], ['nextest', 1, 1], ['cargo', 2, 1], ['nextest', 2, 1],
        ['nextest', 2, 2], ['cargo', 2, 2], ['nextest', 1, 2], ['cargo', 1, 2],
      ]);
      assert.ok(report.runs.every(row => row.returncode === 0 && existsSync(join(output, row.log))));
      const refused = spawnSync(process.execPath, args, { cwd: directory, env, encoding: 'utf8' });
      assert.notEqual(refused.status, 0);
      assert.match(refused.stderr, /EEXIST/);
    }
    const output = join(directory, 'failure');
    const result = spawnSync(process.execPath, [script, '--suite', 'standard', '--nextest-bin', process.env.NEXTEST_BIN, '--output', output], {
      cwd: directory, env: { ...env, PROBE_FAIL: '1' }, encoding: 'utf8',
    });
    assert.notEqual(result.status, 0);
    const report = JSON.parse(readFileSync(join(output, 'results.json'), 'utf8'));
    assert.equal(report.runs.length, 1);
    assert.equal(report.runs[0].returncode, 101);
    const log = readFileSync(join(output, report.runs[0].log), 'utf8');
    assert.match(log, /probe stdout/);
    assert.match(log, /probe stderr/);
    assert.match(log, /ordinary.*FAILED/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
