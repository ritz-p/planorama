'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const fixtures = require('../../layout-quality/manifest.json').fixtures;

const [baseline, experiment, output] = process.argv.slice(2);
if (!baseline || !experiment || !output) throw Error('Expected BASELINE EXPERIMENT OUTPUT');
fs.mkdirSync(output, { recursive: true });

function render(binary, args, prefix, bounded, trace) {
  const env = { ...process.env };
  delete env.PLANORAMA_BOUNDED;
  delete env.PLANORAMA_SHORTCUT_TRACE;
  if (bounded) env.PLANORAMA_BOUNDED = '1';
  if (trace) env.PLANORAMA_SHORTCUT_TRACE = '1';
  const log = fs.openSync(prefix + '.log', 'w');
  const start = process.hrtime.bigint();
  try {
    const result = spawnSync(binary, [...args, '--layout-diagnostics-json', prefix + '.json', '-o', prefix + '.svg'], {
      env, stdio: ['ignore', log, log],
    });
    if (result.error) throw result.error;
    if (result.status !== 0) throw Error('Render failed: ' + prefix);
    return Number(process.hrtime.bigint() - start) / 1e6;
  } finally {
    fs.closeSync(log);
  }
}

for (const { id, args } of fixtures) {
  const prefix = name => path.join(output, id + '-' + name);
  const samples = { first: [], bounded: [] };
  const evaluations = {};
  for (const name of Object.keys(samples)) {
    render(experiment, args, prefix(name), name === 'bounded', true);
    const log = fs.readFileSync(prefix(name) + '.log', 'utf8');
    evaluations[name] = Array.from(log.matchAll(/^EVAL (\d+)$/gm), m => Number(m[1])).reduce((a, b) => a + b, 0);
  }
  for (let repetition = -1; repetition < 3; repetition++) {
    const order = repetition % 2 === 0 ? ['first', 'bounded'] : ['bounded', 'first'];
    for (const name of order) {
      const bounded = name === 'bounded';
      const time = render(bounded ? experiment : baseline, args, prefix('timed-' + name), bounded, false);
      if (repetition >= 0) samples[name].push(time);
    }
  }
  for (const name of Object.keys(samples)) {
    for (const ext of ['.json', '.svg']) {
      if (!fs.readFileSync(prefix(name) + ext).equals(fs.readFileSync(prefix('timed-' + name) + ext))) {
        throw Error('Trace changed output: ' + id + ' ' + name + ext);
      }
    }
  }
  console.log(JSON.stringify({ id, milliseconds: samples, evaluations }));
}
