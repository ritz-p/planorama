const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

function run(suite, fail = '') {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'wasm-ci-'));
  try {
    fs.mkdirSync(path.join(root, 'scripts'));
    fs.mkdirSync(path.join(root, 'bin'));
    fs.mkdirSync(path.join(root, 'browser'));
    for (const script of ['wasm-ci.sh', 'wasm-build-timing.sh', 'build-wasm-distributions.sh', 'build-wasm-bindings.sh', 'assemble-browser.sh']) {
      fs.copyFileSync(path.join(__dirname, '../../scripts', script), path.join(root, 'scripts', script));
    }
    for (const file of ['index.html', 'app.js', 'worker.js']) fs.writeFileSync(path.join(root, 'browser', file), 'UI');
    const stub = `#!/bin/sh
set -eu
name=$(basename "$0")
printf '%s %s\\n' "$name" "$*" >> "$COMMAND_LOG"
case "$name $*" in
  'wasm-bindgen --version') echo 'wasm-bindgen 0.2.100' ;;
  'cargo build '*)
    test "\${FAIL_COMPILE:-}" != yes || exit 23
    mkdir -p "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release"
    echo source.rs > "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/planorama.d"
    ;;
  'wasm-bindgen --target '*)
    while [ "$1" != --out-dir ]; do shift; done
    mkdir -p "$2"
    echo bindings > "$2/planorama.js"
    echo wasm > "$2/planorama_bg.wasm"
    ;;
  'node scripts/prepare-wasm-memory.cjs '*)
    echo adapter >> "$2/planorama.js"
    echo memory > "$2/memory-capabilities.json"
    ;;
esac
`;
    for (const name of ['cargo', 'rustc', 'wasm-bindgen', 'sccache', 'node', 'chromium', 'git', 'du']) {
      fs.writeFileSync(path.join(root, 'bin', name), stub, {mode: 0o755});
    }
    const result = spawnSync('sh', ['scripts/wasm-ci.sh'], {
      cwd: root, encoding: 'utf8', env: {...process.env, PATH: `${root}/bin:${process.env.PATH}`,
        COMMAND_LOG: `${root}/commands`, CARGO_TARGET_DIR: `${root}/target`, WASM_CI_SUITE: suite, FAIL_COMPILE: fail},
    });
    const commands = fs.existsSync(`${root}/commands`) ? fs.readFileSync(`${root}/commands`, 'utf8').split('\n') : [];
    const artifacts = {};
    for (const file of ['planorama.js', 'planorama_bg.wasm', 'memory-capabilities.json']) {
      if (result.status === 0) {
        artifacts[file] = fs.readFileSync(`${root}/output/browser/pkg/${file}`, 'utf8');
        assert.equal(artifacts[file], fs.readFileSync(`${root}/output/wasm-engine/${file}`, 'utf8'));
      }
    }
    return {result, commands, artifacts};
  } finally {
    fs.rmSync(root, {recursive: true, force: true});
  }
}

for (const suite of ['pr', 'full']) {
  test(`${suite} shares one native build and one adapted Wasm build across both distributions`, () => {
    const {result, commands, artifacts} = run(suite);
    assert.equal(result.status, 0, result.stderr);
    const builds = commands.filter(line => line.startsWith('cargo build '));
    assert.equal(builds.length, 2);
    assert.ok(builds.every(line => line.includes('--no-default-features')));
    assert.equal(commands.filter(line => line.startsWith('wasm-bindgen --target')).length, 1);
    assert.equal(commands.filter(line => line.startsWith('node scripts/prepare-wasm-memory.cjs')).length, 1);
    assert.equal(artifacts['planorama.js'], 'bindings\nadapter\n');
    assert.ok(commands.includes('node tests/browser/wasm.cjs'));
    assert.ok(commands.includes('node tests/browser/wasm-engine.cjs'));
    assert.equal(commands.some(line => line.startsWith('cargo test ')), suite === 'full');
  });
}

test('invalid scope and failed compilation cannot pass as skipped validation', () => {
  assert.equal(run('typo').result.status, 2);
  const {result, commands} = run('pr', 'yes');
  assert.notEqual(result.status, 0);
  assert.equal(commands.some(line => line.startsWith('node tests/browser/')), false);
  assert.equal(commands.some(line => line.startsWith('node scripts/package-wasm-engine')), false);
});

test('release branch updates select the full suite without enabling publication', () => {
  const workflow = fs.readFileSync(path.join(__dirname, '../../.github/workflows/wasm-release.yml'), 'utf8');
  assert.match(workflow, /push:\s+branches: \[release\]/);
  assert.match(workflow, /WASM_CI_SUITE=full dev sh scripts\/wasm-ci\.sh/);
  assert.match(workflow, /if: github\.event_name == 'workflow_dispatch' && inputs\.publish/);
  assert.match(workflow, /requireLayoutQuality\(github, context\.repo, commit\)/);
});

test('direct browser verification defaults to full and rejects unknown suites', () => {
  const script = path.join(__dirname, '../browser/wasm-suite.cjs');
  for (const [suite, expected] of [['', true], ['pr', false], ['full', true]]) {
    const result = spawnSync(process.execPath, ['-e', 'console.log(require(process.argv[1]).full)', script], {
      encoding: 'utf8', env: {...process.env, WASM_CI_SUITE: suite},
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout.trim(), String(expected));
  }
  assert.notEqual(spawnSync(process.execPath, [script], {env: {...process.env, WASM_CI_SUITE: 'typo'}}).status, 0);
});
