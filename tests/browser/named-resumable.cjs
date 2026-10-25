const assert = require('node:assert/strict');
const fs = require('node:fs');

module.exports = binding => {
  const json = fs.readFileSync('tests/fixtures/browser-named.json', 'utf8');
  const inputs = JSON.stringify([{state:'a', json}, {state:'b', json}]);
  const invalid = code => error => JSON.parse(String(error)).code === code;
  for (const named of [false, true]) {
    for (const format of ['svg', 'html']) {
      for (const selection of [{}, {modules:[named ? 'a:module.app' : 'module.app']}, {focus:(named ? 'a:' : '') + 'module.app.terraform_data.item'}]) {
        const options = JSON.stringify({format, ...selection, html_diagnostics:format === 'html', html_semantic_diagnostics:format === 'html'});
        const create = () => named ? binding.Generation.named(inputs, options) : new binding.Generation(json, options);
        const expected = named ? binding.generate_named(inputs, options) : binding.generate(json, options);
        for (const units of [1, 4096]) {
          const job = create(), stages = [];
          try {
            assert.equal(job.stage(), 'parsing');
            for (const bad of [0, 4097, 1.5, NaN, Infinity, -1, 4294967297]) {
              assert.throws(() => job.step(bad), invalid('invalid_options'));
              assert.throws(() => job.step_with_progress(bad, () => stages.push('bad')), invalid('invalid_options'));
            }
            let actual, steps = 0;
            do {
              actual = job.step_with_progress(units, stage => { stages.push(stage); throw new Error('ignored observer error'); });
              assert.ok(++steps < 1000000);
            } while (actual === undefined);
            assert.equal(actual, expected);
            assert.equal(stages[0], 'parsing');
            assert.equal(stages.at(-1), 'output');
            assert.equal(job.stage(), 'output');
            assert.ok(stages.indexOf('selection') < stages.indexOf('placement'));
            assert.ok(stages.indexOf('placement') < stages.indexOf('routing'));
            assert.ok(stages.every(stage => ['parsing','selection','placement','routing','output'].includes(stage)));
            assert.throws(() => job.step(1), invalid('generation_closed'));
          } finally { job.free(); }
        }
        const cancelled = create();
        cancelled.step(1); cancelled.cancel(); cancelled.cancel();
        assert.throws(() => cancelled.step(1), invalid('generation_closed'));
        cancelled.free();
        const retry = create();
        try { let result; do { result = retry.step(4096); } while (result === undefined); assert.equal(result, expected); }
        finally { retry.free(); }
      }
    }
  }
  for (const [value, code] of [
    [[], 'empty_inputs'],
    [[{state:'a',json},{state:'a',json}], 'duplicate_state_name'],
    [[{state:'a:b',json}], 'invalid_state_name'],
    [['a','b'].map(state => ({state,json:' '.repeat(1024*1024+1)})), 'resource_limit'],
  ]) assert.throws(() => binding.Generation.named(JSON.stringify(value), '{}'), invalid(code));
  assert.throws(() => binding.Generation.named(inputs, ' '.repeat(65537)), invalid('resource_limit'));
  assert.throws(() => binding.Generation.named(' '.repeat(2*1024*1024*6+16385), '{}'), invalid('resource_limit'));
  const failed = binding.Generation.named(JSON.stringify([{state:'a',json:'invalid secret'}]), '{}');
  try {
    assert.throws(() => failed.step(4096), invalid('invalid_json'));
    assert.throws(() => failed.step(1), invalid('generation_closed'));
  } finally { failed.free(); }
  const structure = JSON.stringify({format_version:'1.2', resource_changes:[], extra:Array(17000).fill(0)});
  const bounded = binding.Generation.named(JSON.stringify(['a','b'].map(state => ({state,json:structure}))), '{}');
  try {
    assert.throws(() => bounded.step(4096), invalid('resource_limit'));
    assert.throws(() => bounded.step(1), invalid('generation_closed'));
  } finally { bounded.free(); }
};
