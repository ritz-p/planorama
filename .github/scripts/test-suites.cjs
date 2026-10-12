const {spawnSync} = require('node:child_process');

const LIBRARY_KINDS = new Set(['lib', 'rlib', 'dylib', 'cdylib', 'staticlib', 'proc-macro']);
const TARGET_KINDS = new Set(['bin', 'test', 'example', 'bench']);

function targetKind(target) {
  if (!Array.isArray(target.kind) || !target.kind.length) throw new Error('Missing Cargo target kind.');
  if (target.kind.every(kind => LIBRARY_KINDS.has(kind))) return 'lib';
  if (target.kind.length === 1 && (TARGET_KINDS.has(target.kind[0]) || target.kind[0] === 'custom-build')) {
    return target.kind[0];
  }
  throw new Error(`Unsupported Cargo target kind: ${target.kind.join(', ')}.`);
}

function suiteCommands(metadata, suite) {
  if (!['standard', 'large'].includes(suite)) throw new Error(`Unknown test suite: ${suite}.`);
  if (!Array.isArray(metadata.packages) || !Array.isArray(metadata.workspace_default_members)
      || !metadata.workspace_default_members.length) {
    throw new Error('Cargo metadata must identify the default workspace packages.');
  }
  const targets = metadata.workspace_default_members.flatMap(id => {
    const pkg = metadata.packages.find(candidate => candidate.id === id);
    if (!pkg || typeof pkg.name !== 'string' || !Array.isArray(pkg.targets)) {
      throw new Error(`Missing Cargo package metadata: ${id}.`);
    }
    return pkg.targets.map(target => {
      const kind = targetKind(target);
      if (typeof target.name !== 'string' || !target.name) throw new Error('Missing Cargo target name.');
      return {package: id, kind, name: target.name,
        suite: pkg.name === 'planorama' && kind === 'test' && target.name === 'large_example' ? 'large' : 'standard'};
    }).filter(target => target.kind !== 'custom-build');
  });
  if (targets.filter(target => target.suite === 'large').length !== 1) {
    throw new Error('Expected exactly one planorama/large_example integration target; update the suite policy.');
  }
  const commands = metadata.workspace_default_members.flatMap(id => {
    const selected = targets.filter(target => target.package === id && target.suite === suite);
    if (!selected.length) return [];
    const selectors = selected.flatMap(target => target.kind === 'lib' ? ['--lib'] : [`--${target.kind}`, target.name]);
    return [['test', '--locked', '--all-features', '--package', id, ...selectors]];
  });
  if (!commands.length) throw new Error(`No Cargo targets selected for the ${suite} suite.`);
  return commands;
}

function main() {
  const [suite, phase, ...extra] = process.argv.slice(2);
  if (!['standard', 'large'].includes(suite) || !['compile', 'run'].includes(phase) || extra.length) {
    throw new Error('Usage: node .github/scripts/test-suites.cjs <standard|large> <compile|run>');
  }
  const metadata = spawnSync('cargo', ['metadata', '--locked', '--all-features', '--no-deps', '--format-version', '1'], {
    encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 16 * 1024 * 1024,
  });
  if (metadata.error) throw metadata.error;
  if (metadata.status !== 0) process.exit(metadata.status ?? 1);
  for (const command of suiteCommands(JSON.parse(metadata.stdout), suite)) {
    const args = phase === 'compile' ? [...command, '--no-run'] : command;
    console.log(`Suite ${suite}: cargo ${args.map(argument => JSON.stringify(argument)).join(' ')}`);
    const result = spawnSync('cargo', args, {stdio: 'inherit'});
    if (result.error) throw result.error;
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}

module.exports = {suiteCommands};

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
