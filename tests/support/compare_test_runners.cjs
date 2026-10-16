const assert = require('node:assert/strict');
const { execFileSync, spawnSync } = require('node:child_process');
const { mkdirSync, writeFileSync, readFileSync, openSync, closeSync, rmSync } = require('node:fs');
const { resolve, join, delimiter } = require('node:path');
const { cpus, constants } = require('node:os');
const { performance } = require('node:perf_hooks');
const { suiteCommands } = require('../../.github/scripts/test-suites.cjs');

const capture = command => execFileSync(command[0], command.slice(1), { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });

function inventory(selectors) {
  const data = JSON.parse(capture(['cargo', 'nextest', 'list', ...selectors, '--message-format', 'json']));
  const result = {};
  for (const [binary, suite] of Object.entries(data['rust-suites'])) {
    const listed = (...options) => capture([suite['binary-path'], '--list', ...options])
      .split(/\r?\n/).filter(line => line.endsWith(': test')).map(line => line.slice(0, -6)).sort();
    const cases = suite.testcases;
    const ignored = Object.keys(cases).filter(name => cases[name].ignored).sort();
    assert.deepEqual(listed(), Object.keys(cases).sort(), `${binary}: Cargo and nextest test names differ`);
    assert.deepEqual(listed('--ignored'), ignored, `${binary}: ignored test lists differ`);
    for (const entry of Object.values(cases)) {
      assert.deepEqual(entry['filter-match'], entry.ignored ? { status: 'mismatch', reason: 'ignored' } : { status: 'matches' }, `${binary}: unexpected nextest default filter`);
    }
    result[binary] = { tests: Object.keys(cases).filter(name => !cases[name].ignored).sort(), ignored };
  }
  if (!Object.values(result).some(suite => suite.tests.length)) throw new Error('No ordinary tests selected');
  return result;
}

function parseArgs(argv) {
  const args = { threads: [2, 8], repeats: 2 };
  for (let i = 0; i < argv.length; i++) {
    const key = argv[i];
    if (key === '--threads') {
      args.threads = [];
      while (i + 1 < argv.length && !argv[i + 1].startsWith('--')) args.threads.push(Number(argv[++i]));
    } else if (['--suite', '--repeats', '--nextest-bin', '--output'].includes(key)) {
      if (!argv[i + 1] || argv[i + 1].startsWith('--')) throw new Error(`Missing value for ${key}`);
      args[key.slice(2)] = argv[++i];
    } else throw new Error(`Unknown argument: ${key}`);
  }
  args.repeats = Number(args.repeats);
  if (!['standard', 'large'].includes(args.suite) || !args['nextest-bin'] || !args.output) {
    throw new Error('Required: --suite standard|large --nextest-bin DIRECTORY --output DIRECTORY');
  }
  if (!Number.isInteger(args.repeats) || args.repeats < 2 || !args.threads.length || args.threads.some(n => !Number.isInteger(n) || n < 1)) {
    throw new Error('Use at least two repeats and positive integer thread counts');
  }
  return args;
}

function measure(command, log, timing) {
  const stream = openSync(log, 'w');
  const started = performance.now();
  let processResult;
  try {
    processResult = spawnSync('/usr/bin/time', ['-f', '%U %S', '-o', timing, '--', ...command], { stdio: ['ignore', stream, stream] });
  } finally { closeSync(stream); }
  const wall = (performance.now() - started) / 1000;
  if (processResult.error) throw processResult.error;
  let cpu, childSignal;
  try {
    const timingText = readFileSync(timing, 'utf8').trim();
    childSignal = timingText.match(/Command terminated by signal (\d+)/)?.[1];
    const line = timingText.split('\n').at(-1);
    if (!/^\d+(?:\.\d+)? \d+(?:\.\d+)?$/.test(line)) throw new Error(`Invalid CPU timing: ${line}`);
    cpu = line.split(' ').map(Number).reduce((a, b) => a + b, 0);
  } finally { rmSync(timing, { force: true }); }
  const round = n => Number(n.toFixed(3));
  return {
    wall_seconds: round(wall), cpu_seconds: round(cpu), average_cpu_cores: round(cpu / wall),
    returncode: childSignal ? -Number(childSignal) : processResult.signal ? -constants.signals[processResult.signal] : processResult.status,
  };
}

function main(argv) {
  const args = parseArgs(argv);
  process.env.PATH = resolve(args['nextest-bin']) + delimiter + process.env.PATH;
  process.env.CARGO_TERM_COLOR = 'never';
  mkdirSync(resolve(args.output, '..'), { recursive: true });
  mkdirSync(args.output);
  const metadata = JSON.parse(capture(['cargo', 'metadata', '--locked', '--all-features', '--no-deps', '--format-version', '1']));
  const commands = suiteCommands(metadata, args.suite);
  if (commands.length !== 1) throw new Error('This comparison requires one default workspace package');
  const selectors = commands[0].slice(1);
  execFileSync('cargo', ['test', ...selectors, '--no-run'], { stdio: 'inherit' });
  const tests = inventory(selectors);
  const report = {
    revision: capture(['git', 'rev-parse', 'HEAD']).trim(), working_tree: capture(['git', 'status', '--short']),
    rustc: capture(['rustc', '--version', '--verbose']), nextest: capture(['cargo', 'nextest', '--version']),
    logical_cpus: cpus().length, suite: args.suite, inventory: tests, runs: [],
  };
  const destination = join(args.output, 'results.json');
  const save = () => writeFileSync(destination, JSON.stringify(report, null, 2) + '\n');
  save();
  for (let repeat = 1; repeat <= args.repeats; repeat++) {
    const settings = repeat % 2 ? args.threads : [...args.threads].reverse();
    for (const threads of settings) {
      for (const runner of repeat % 2 ? ['cargo', 'nextest'] : ['nextest', 'cargo']) {
        const command = runner === 'cargo'
          ? ['cargo', 'test', ...selectors, '--', `--test-threads=${threads}`]
          : ['cargo', 'nextest', 'run', ...selectors, '--test-threads', String(threads), '--no-fail-fast', '--status-level', 'fail', '--final-status-level', 'fail'];
        const log = `${runner}-${threads}-${repeat}.log`;
        const row = { runner, threads, repeat, ...measure(command, join(args.output, log), join(args.output, '.timing')), command, log };
        report.runs.push(row);
        save();
        console.log(JSON.stringify(row));
        if (row.returncode) throw new Error(`Test run failed; inspect ${join(args.output, log)}`);
      }
    }
  }
}

if (require.main === module) {
  try { main(process.argv.slice(2)); }
  catch (error) { console.error(error); process.exitCode = 1; }
}
module.exports = { parseArgs, inventory, measure, main };
