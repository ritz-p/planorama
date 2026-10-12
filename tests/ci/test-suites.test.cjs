const assert = require('node:assert/strict');
const {test} = require('node:test');
const {suiteCommands} = require('../../.github/scripts/test-suites.cjs');

const ROOT = 'path+file:///workspace#planorama@0.1.0';
const target = (name, kind, extra = {}) => ({name, kind: [kind], ...extra});

function metadata() {
  return {
    workspace_default_members: [ROOT],
    packages: [{id: ROOT, name: 'planorama', targets: [
      target('build-script-build', 'custom-build'), target('planorama', 'bin'),
      target('cli', 'test'), target('large_example', 'test'), target('rendering', 'test'),
      target('terraform_plan', 'test'), target('generate_large_fixture', 'example', {test: false}),
    ]}],
  };
}

test('partitions all current unit, integration and example targets without overlap', () => {
  const data = metadata();
  assert.deepEqual(suiteCommands(data, 'standard'), [[
    'test', '--locked', '--all-features', '--package', ROOT,
    '--bin', 'planorama', '--test', 'cli', '--test', 'rendering', '--test', 'terraform_plan',
    '--example', 'generate_large_fixture',
  ]]);
  assert.deepEqual(suiteCommands(data, 'large'), [[
    'test', '--locked', '--all-features', '--package', ROOT, '--test', 'large_example',
  ]]);
});

test('automatically includes new integration targets, bins, libraries, examples and benches', () => {
  const data = metadata();
  data.packages[0].targets.push(
    target('new_integration', 'test'), target('second_cli', 'bin', {test: false}),
    target('planorama_core', 'lib'), target('new_example', 'example', {test: false}),
    target('new_benchmark', 'bench', {test: false}), target('disabled_test', 'test', {test: false}),
  );
  const [command] = suiteCommands(data, 'standard');
  assert.deepEqual(command.slice(-11), [
    '--test', 'new_integration', '--bin', 'second_cli', '--lib',
    '--example', 'new_example', '--bench', 'new_benchmark', '--test', 'disabled_test',
  ]);
  assert.equal(command.includes('large_example'), false);
});

test('preserves Cargo default workspace selection and leaves dependency targets unselected', () => {
  const data = metadata();
  const other = 'path+file:///workspace/other#other@0.1.0';
  data.packages.push({id: other, name: 'other', targets: [target('large_example', 'test'), target('other', 'proc-macro')]});
  data.packages.push({id: 'dependency', name: 'dependency', targets: [target('not_a_workspace_target', 'bin')]});
  assert.equal(suiteCommands(data, 'standard').length, 1);
  data.workspace_default_members.push(other);
  const commands = suiteCommands(data, 'standard');
  assert.equal(commands.length, 2);
  assert.deepEqual(commands[1], [
    'test', '--locked', '--all-features', '--package', other, '--test', 'large_example', '--lib',
  ]);
  assert.equal(suiteCommands(data, 'large').length, 1);
});

test('recognizes library targets with multiple supported crate types', () => {
  const data = metadata();
  data.packages[0].targets.push({name: 'core', kind: ['rlib', 'cdylib']});
  assert.equal(suiteCommands(data, 'standard')[0].at(-1), '--lib');
});

test('fails when the heavy target is renamed instead of silently running an empty suite', () => {
  const data = metadata();
  data.packages[0].targets.find(item => item.name === 'large_example').name = 'renamed_large_example';
  assert.throws(() => suiteCommands(data, 'large'), /exactly one planorama\/large_example/);
  assert.throws(() => suiteCommands(data, 'standard'), /exactly one planorama\/large_example/);
});

test('rejects missing or ambiguous heavy targets and unsupported target kinds', () => {
  const ambiguous = metadata();
  ambiguous.packages[0].targets.push(target('large_example', 'test'));
  assert.throws(() => suiteCommands(ambiguous, 'large'), /exactly one/);
  const unknown = metadata();
  unknown.packages[0].targets.push(target('future_target', 'unknown'));
  assert.throws(() => suiteCommands(unknown, 'standard'), /Unsupported Cargo target kind/);
});

test('rejects unknown suites and incomplete default package metadata', () => {
  assert.throws(() => suiteCommands(metadata(), 'typo'), /Unknown test suite/);
  assert.throws(() => suiteCommands({packages: []}, 'standard'), /default workspace packages/);
  const data = metadata();
  data.workspace_default_members.push('missing');
  assert.throws(() => suiteCommands(data, 'standard'), /Missing Cargo package metadata/);
});
