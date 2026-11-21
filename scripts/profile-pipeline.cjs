const fs = require('node:fs');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {createHash} = require('node:crypto');
const {performance} = require('node:perf_hooks');

const [engine, artifact, fixture, format] = process.argv.slice(2);
const workloads = {
  typical: {input: 'examples/plan.json'},
  dense: {input: 'examples/terraform-large/plan.json'},
  'multi-state': {
    states: {consumer: 'tests/fixtures/multi/consumer.json', producer: 'tests/fixtures/multi/producer.json'},
    mappings: [['consumer', 'data.terraform_remote_state.network', 'producer']],
  },
  'multi-account': {
    states: Object.fromEntries(['application-a', 'application-b', 'shared-services']
      .map(state => [state, `examples/multi-account/${state}.json`])),
    mappings: [
      ['application-a', 'module.application.data.terraform_remote_state.shared', 'shared-services'],
      ['application-b', 'module.application.data.terraform_remote_state.shared', 'shared-services'],
      ['application-b', 'data.terraform_remote_state.peer', 'application-a'],
    ],
  },
};
if (!['native', 'wasm'].includes(engine) || !artifact || !Object.hasOwn(workloads, fixture) ||
    !['html', 'svg'].includes(format)) {
  throw new Error('Usage: node scripts/profile-pipeline.cjs native|wasm BINARY_OR_BINDINGS typical|dense|multi-state|multi-account svg|html');
}
const file = path.resolve(artifact);
process.chdir(path.join(__dirname, '..'));
const workload = workloads[fixture];
const files = workload.input ? [workload.input] : Object.values(workload.states);
const inputs = files.map(name => {
  const bytes = fs.readFileSync(name);
  return {file: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex')};
});

function native() {
  const args = workload.input ? [workload.input] : Object.entries(workload.states)
    .flatMap(([state, name]) => ['--state', state + '=' + name]);
  for (const [consumer, remote, producer] of workload.mappings || [])
    args.push('--remote-state', consumer + ':' + remote + '=' + producer);
  args.push('--timings', '--format', format, '--icon-mode', 'service-name', '-o', '-');
  const result = spawnSync(file, args, {stdio: ['ignore', 'ignore', 'pipe'], encoding: 'utf8'});
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr || 'native generation failed');
  const total = result.stderr.match(/^timings: total_ms=([\d.]+)/m);
  if (!total) throw new Error('missing native timing report');
  const stages = result.stderr.split('\n').filter(line => line.startsWith('timing: ')).map(line =>
    Object.fromEntries(line.slice(8).split(' ').map(field => {
      const [key, value] = field.split('=');
      return [key, key === 'stage' ? value : Number(value)];
    })));
  return {total_ms: Number(total[1]), stages};
}

function wasm() {
  const binding = new Function(fs.readFileSync(path.join(file, 'planorama.js'), 'utf8') + '\nreturn wasm_bindgen;')();
  const instance = binding.initSync({module: fs.readFileSync(path.join(file, 'planorama_bg.wasm')), memory_budget_mib: 512});
  const options = JSON.stringify({format, icon_mode: 'service-name', remote_state_mappings:
    (workload.mappings || []).map(([consumer, remote, producer]) => ({consumer, remote, producer}))});
  const input = workload.input ? fs.readFileSync(workload.input, 'utf8') :
    JSON.stringify(Object.entries(workload.states).map(([state, name]) => ({state, json: fs.readFileSync(name, 'utf8')})));
  const stages = [];
  const spans = [], pending = [];
  binding.set_profile_callback?.((stage, enter) => {
    const now = performance.now(), usage = process.cpuUsage();
    if (enter) {
      const id = spans.length + 1;
      spans.push({id, parent: pending.at(-1)?.id ?? 0, stage});
      pending.push({id, time: now, usage});
    } else {
      const start = pending.pop();
      const row = spans[start.id - 1];
      if (row.stage !== stage) throw new Error('unbalanced Wasm timing spans');
      Object.assign(row, {inclusive_ms: now - start.time,
        process_cpu_ms: (usage.user - start.usage.user + usage.system - start.usage.system) / 1000,
        linear_memory_bytes_at_end: instance.memory.buffer.byteLength,
        process_peak_rss_kib_at_end: process.resourceUsage().maxRSS});
    }
  });
  const started = performance.now();
  let time = started, cpu = process.cpuUsage(), stage = 'input_transfer';
  function boundary(next) {
    const now = performance.now(), usage = process.cpuUsage();
    stages.push({stage, elapsed_ms: now - time,
      process_cpu_ms: (usage.user - cpu.user + usage.system - cpu.system) / 1000,
      linear_memory_bytes_at_end: instance.memory.buffer.byteLength,
      process_peak_rss_kib_at_end: process.resourceUsage().maxRSS});
    time = now;
    cpu = usage;
    stage = next;
  }
  const generate = workload.input ? binding.generate_with_progress : binding.generate_named_with_progress;
  const result = generate(input, options, boundary);
  boundary('complete');
  binding.set_profile_callback?.(undefined);
  return {total_ms: time - started, result_bytes: Buffer.byteLength(result), stages, spans};
}

const measurement = engine === 'native' ? native() : wasm();
console.log(JSON.stringify({fixture, format, engine, node: process.version,
  platform: process.platform, arch: process.arch, inputs, ...measurement}, null, 2));
