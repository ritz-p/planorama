const {execFileSync, spawnSync} = require('node:child_process');
const {mkdirSync, readFileSync} = require('node:fs');
const {constants} = require('node:os');
const {pilots, filterFor} = require('./rust-mutant-filter.cjs');

function run(args) {
  const [name = 'reference'] = args;
  if (args.length > 1 || !Object.hasOwn(pilots, name)) {
    console.error('Usage: sh scripts/rust-mutants.sh [' + Object.keys(pilots).join('|') + ']');
    return 1;
  }
  const capture = {encoding: 'utf8', stdio: ['inherit', 'pipe', 'inherit']};
  const version = spawnSync('cargo', ['mutants', '--version'], capture);
  if (version.status !== 0 || version.stdout.trimEnd() !== 'cargo-mutants 25.3.1') {
    console.error('Install cargo-mutants 25.3.1 before running the pilot.');
    return 1;
  }
  const pilot = pilots[name];
  const expected = pilot.expected ?? 1;
  const output = pilot.output ?? 'output/rust-mutants/' + name;
  const sources = pilot.sources ?? [pilot.source];
  const targets = pilot.targets ?? ['--lib'];
  const selectionArgs = ['mutants', '-j', '1', '--no-shuffle', '--build-timeout', '420', '--timeout', '90', '--all-features', '-C=--locked',
    ...targets.map(target => '-C=' + target), ...sources.flatMap(source => ['-f', source]), '-F', filterFor(pilot)];
  const env = {...process.env, CARGO_PROFILE_TEST_OPT_LEVEL: '0', CARGO_PROFILE_TEST_DEBUG: '0'};
  const selection = execFileSync('cargo', [...selectionArgs, '--list'], {...capture, env}).trimEnd();
  const count = selection.split('\n').filter(Boolean).length;
  if (count !== expected) {
    console.error(`Expected ${expected} ${name} mutations, found ${count}:\n${selection}`);
    return 1;
  }
  console.log(selection);
  mkdirSync(output, {recursive: true});
  const result = spawnSync('timeout', ['-k', '10s', '20m', 'cargo', ...selectionArgs, '-o', output, '--', '--', ...pilot.tests], {stdio: 'inherit', env});
  if (result.error) throw result.error;
  const status = result.status ?? 128 + constants.signals[result.signal];
  if (status === 0 || status === 2) {
    const log = readFileSync(output + '/mutants.out/log/baseline.log', 'utf8');
    const passed = log.split('\n').filter(line => /^test result: ok\. [1-9][0-9]* passed;/.test(line));
    if (!passed.length) {
      console.error('The ' + name + ' baseline must execute at least one passing test.');
      return 1;
    }
    console.log(passed.join('\n'));
  }
  return status;
}

try {
  process.exitCode = run(process.argv.slice(2));
} catch (error) {
  console.error(error.message);
  process.exitCode = error.status || 1;
}
