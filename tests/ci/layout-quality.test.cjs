const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {spawnSync} = require('node:child_process');
const {compare, markdown, run} = require('../../scripts/layout-quality.cjs');
const manifest = require('../layout-quality/manifest.json');
const baseline = () => JSON.parse(fs.readFileSync(path.join(__dirname, '../layout-quality/fan-in.json'))).report;
const budgets = manifest.fixtures.find(f => f.id === 'fan-in').budgets;

test('independent crossing, overlap and clearance degradations fail with stable relationships', () => {
  for (const [metric, mutate, category] of [
    ['crossings', r => r.summary.visible_straight_sections.crossings++, 'crossing'],
    ['overlap', r => r.summary.visible_straight_sections.independent_overlap_length++, 'independent_overlap'],
    ['border', r => r.summary.visible_border_minimum_deficit++, 'visible_border_clearance'],
  ]) {
    const before = baseline();
    const after = baseline();
    mutate(after);
    after.findings.push({category, edge: after.edges[0].id, peer: after.edges[1].id,
      location: {interval: [0, 1]}});
    const result = {fixture: 'degraded', ...compare(before, after, budgets)};
    assert.equal(result.status, 'failed');
    const text = markdown([result]);
    assert.ok(text.includes(`FAIL: ${metric}`));
    assert.ok(text.includes(after.edges[0].id));
    assert.ok(text.includes('delta 1'));
  }
});

test('deterministic reports and explicitly budgeted changes pass; area and raw length are informational', () => {
  const before = baseline();
  assert.deepEqual(compare(before, baseline(), budgets), {status: 'passed', changes: []});
  const after = baseline();
  after.edges[0].raw_polyline.bends += 2;
  after.summary.visible_border_minimum_deficit += 0.25;
  after.canvas.width *= 2;
  after.edges[0].raw_polyline.length *= 10;
  assert.equal(compare(before, after, budgets).status, 'passed');
  after.edges[0].raw_polyline.bends++;
  assert.equal(compare(before, after, budgets).status, 'failed');
  const relative = structuredClone(budgets);
  relative.length = {mode: 'gate', absolute: 0, relative: 0.1};
  const slight = baseline();
  slight.edges[0].raw_polyline.length *= 1.1;
  assert.equal(compare(before, slight, relative).status, 'passed');
  slight.edges[0].raw_polyline.length += 1;
  assert.equal(compare(before, slight, relative).status, 'failed');
});

test('schema, policy, identity, ordering, numeric and incomplete reports reject comparison', () => {
  for (const mutate of [
    r => r.schema_version++, r => r.policy.border_minimum_gap++,
    r => r.edges.reverse(), r => r.edges.pop(),
    r => { r.summary.visible_straight_sections.crossings = NaN; },
    r => { r.limits.findings_complete = false; },
    r => { r.limits.pair_metrics_complete = false; },
    r => r.limits.unsupported_diagonal_segments++,
    r => { delete r.findings; },
    r => { r.findings = null; },
    r => { r.findings = {}; },
  ]) {
    const after = baseline();
    mutate(after);
    assert.throws(() => compare(baseline(), after, budgets));
  }
});

test('every required summary measurement rejects missing or invalid values on either side', () => {
  const paths = [
    ['routed_paths'], ['raw_polyline', 'length'], ['raw_polyline', 'bends'],
    ['visible_straight_sections', 'crossings'], ['visible_straight_sections', 'independent_overlap_length'],
    ['visible_straight_sections', 'parallel_deficit'], ['visible_straight_sections', 'shared_trunk_length'],
    ['visible_border_minimum_deficit'],
  ];
  for (const keys of paths) {
    for (const value of [undefined, null, '0', -1, NaN, Infinity]) {
      const report = baseline();
      const parent = keys.slice(0, -1).reduce((object, key) => object[key], report.summary);
      if (value === undefined) delete parent[keys.at(-1)];
      else parent[keys.at(-1)] = value;
      assert.throws(() => compare(baseline(), report, budgets));
      assert.throws(() => compare(report, baseline(), budgets));
    }
  }
  for (const key of ['raw_polyline', 'visible_straight_sections']) {
    const report = baseline();
    delete report.summary[key];
    assert.throws(() => compare(baseline(), report, budgets));
  }
});

test('Unicode stable IDs use Rust UTF-8 ordering instead of JavaScript UTF-16 ordering', () => {
  const report = baseline();
  const names = ['\uE000', '\u{10000}'];
  const mapping = new Map(report.edges.map((edge, i) => [edge.id, JSON.stringify({key: names[i] || `z${i}`})]));
  for (const edge of report.edges) edge.id = mapping.get(edge.id);
  report.edges.sort((a, b) => Buffer.compare(Buffer.from(a.id), Buffer.from(b.id)));
  for (const finding of report.findings) {
    finding.edge = mapping.get(finding.edge);
    if (finding.peer) finding.peer = mapping.get(finding.peer);
  }
  assert.deepEqual(compare(report, structuredClone(report), budgets), {status: 'passed', changes: []});
  const wrong = structuredClone(report);
  wrong.edges.sort((a, b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  assert.notDeepEqual(wrong.edges.map(e => e.id), report.edges.map(e => e.id));
  assert.throws(() => compare(report, wrong, budgets), /edge ordering/);
});

test('pair regressions cannot be offset by another pair, and reversed pair order is equivalent', () => {
  for (const [metric, category, amount] of [
    ['crossings', 'crossing', 1], ['overlap', 'independent_overlap', 100],
    ['parallel', 'parallel_proximity', 800],
  ]) {
    const before = baseline();
    const [a, b, c] = before.edges.map(e => e.id);
    before.findings = [{category, edge: a, peer: b, location: {interval: [0, 100], coordinates: [0, 8]}}];
    Object.assign(before.summary.visible_straight_sections, {crossings: 0, independent_overlap_length: 0, parallel_deficit: 0,
      [{crossings: 'crossings', overlap: 'independent_overlap_length', parallel: 'parallel_deficit'}[metric]]: amount});
    assert.throws(() => compare(before, {...before, findings: []}, budgets), /findings total .* differs from summary/);
    const after = structuredClone(before);
    after.findings[0].peer = c;
    const result = compare(before, after, budgets);
    assert.equal(result.status, 'failed');
    const failure = result.changes.find(change => change.failed);
    assert.equal(failure.metric, metric);
    assert.equal(failure.delta, amount);
    assert.deepEqual(new Set([failure.edge, failure.peer]), new Set([a, c]));
    assert.ok(markdown([{fixture: 'pair', ...result}]).includes(c));
    const allowed = structuredClone(budgets);
    allowed[metric].absolute = amount;
    assert.equal(compare(before, after, allowed).status, 'passed');
    after.findings[0].edge = b;
    after.findings[0].peer = a;
    assert.deepEqual(compare(before, after, budgets), {status: 'passed', changes: []});
  }
});

test('incomplete or excess pair findings reject reports even when summaries are unchanged', () => {
  for (const name of ['large', 'multi-state']) {
    const before = JSON.parse(fs.readFileSync(path.join(__dirname, `../layout-quality/${name}.json`))).report;
    for (const category of ['crossing', 'independent_overlap', 'parallel_proximity']) {
      const after = structuredClone(before);
      const finding = after.findings.find(f => f.category === category);
      if (!finding) continue;
      after.findings.splice(after.findings.indexOf(finding), 1);
      assert.throws(() => compare(before, after, budgets), /findings total .* differs from summary/);
      assert.throws(() => compare(after, before, budgets), /findings total .* differs from summary/);
      after.findings.push(finding, finding);
      assert.throws(() => compare(before, after, budgets), /findings total .* differs from summary/);
    }
    const empty = {...before, findings: []};
    assert.throws(() => compare(before, empty, budgets), /findings total .* differs from summary/);
  }
});

test('successful renderer without fresh regular diagnostics cannot check or update a baseline', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-stale-'));
  try {
    const fake = path.join(dir, 'fake.cjs');
    const fixture = {...manifest.fixtures[2], args: [fake]};
    const manifestPath = path.join(dir, 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    const options = {manifestPath, binary: process.execPath, output: path.join(dir, 'out')};
    const source = path.join(__dirname, '../layout-quality/fan-in.json');
    fs.writeFileSync(fake, `const fs = require('node:fs'); fs.writeFileSync(process.argv.at(-1), JSON.stringify(JSON.parse(fs.readFileSync(${JSON.stringify(source)})).report));`);
    assert.equal(run({...options, update: true}), true);
    const saved = fs.readFileSync(path.join(dir, 'fan-in.json'), 'utf8');
    for (const update of [false, true]) {
      fs.writeFileSync(path.join(options.output, 'fan-in.json'), JSON.stringify(baseline()));
      fs.writeFileSync(fake, '');
      assert.equal(run({...options, update}), false);
      assert.equal(fs.readFileSync(path.join(dir, 'fan-in.json'), 'utf8'), saved);
    }
    fs.writeFileSync(fake, `require('node:fs').symlinkSync(${JSON.stringify(source)}, process.argv.at(-1));`);
    assert.equal(run({...options, update: true}), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /regular diagnostics file/);
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('one fixture failure remains visible next to improvements elsewhere', () => {
  const before = baseline();
  const after = baseline();
  after.summary.visible_straight_sections.crossings++;
  after.findings.push({category: 'crossing', edge: after.edges[0].id, peer: after.edges[1].id});
  const text = markdown([{fixture: 'bad', ...compare(before, after, budgets)},
    {fixture: 'good', ...compare(after, before, budgets)}]);
  assert.match(text, /bad: failed/);
  assert.match(text, /good: passed/);
});

test('runner identifies missing and stale baselines and preserves full artifacts', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-quality-'));
  try {
    const fake = path.join(dir, 'fake.cjs');
    fs.writeFileSync(fake, `const fs = require('node:fs'); const r = JSON.parse(fs.readFileSync(${JSON.stringify(path.join(__dirname, '../layout-quality/fan-in.json'))})); fs.writeFileSync(process.argv.at(-1), JSON.stringify(r.report));`);
    const fixture = {...manifest.fixtures[2], args: [fake]};
    const manifestPath = path.join(dir, 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    const options = {manifestPath, binary: process.execPath, output: path.join(dir, 'out')};
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /missing baseline/);
    assert.equal(run({...options, update: true}), true);
    assert.equal(run(options), true);
    fs.appendFileSync(fake, '\nconst report = JSON.parse(fs.readFileSync(process.argv.at(-1))); report.summary.visible_straight_sections.crossings++; report.findings.push({category: "crossing", edge: report.edges[0].id, peer: report.edges[1].id}); fs.writeFileSync(process.argv.at(-1), JSON.stringify(report));');
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /FAIL: crossings/);
    const originalBaseline = fs.readFileSync(path.join(dir, 'fan-in.json'), 'utf8');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture,
      {...fixture, id: 'broken', inputs: [path.join(dir, 'missing.json')]}]}));
    assert.equal(run({...options, update: true}), false);
    assert.equal(fs.readFileSync(path.join(dir, 'fan-in.json'), 'utf8'), originalBaseline);
    fixture.budgets = {...budgets, bends: {mode: 'gate', absolute: 3, relative: 0}};
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /stale baseline/);
    assert.ok(fs.existsSync(path.join(options.output, 'results.json')));
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('reordered resource/configuration inputs retain identical final diagnostics',
  {skip: !process.env.LAYOUT_QUALITY_BINARY}, () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-order-'));
    try {
      const input = JSON.parse(fs.readFileSync('examples/bundling-plan.json'));
      input.resource_changes.reverse();
      input.configuration.root_module.resources.reverse();
      const plan = path.join(dir, 'plan.json');
      fs.writeFileSync(plan, JSON.stringify(input));
      const reports = [];
      for (const [i, file] of ['examples/bundling-plan.json', plan].entries()) {
        const report = path.join(dir, `${i}.json`);
        const child = spawnSync(process.env.LAYOUT_QUALITY_BINARY, [file, '-o', path.join(dir, `${i}.svg`),
          '--layout-diagnostics-json', report], {encoding: 'utf8', timeout: 30000});
        assert.equal(child.status, 0, child.stderr);
        reports.push(JSON.parse(fs.readFileSync(report)));
      }
      assert.deepEqual(reports[0], reports[1]);
      assert.equal(compare(reports[0], reports[1], budgets).status, 'passed');
      const multi = manifest.fixtures.find(f => f.id === 'multi-state');
      const reversed = [...multi.args.slice(2, 4), ...multi.args.slice(0, 2), ...multi.args.slice(4)];
      const states = [];
      for (const [i, args] of [multi.args, reversed].entries()) {
        const report = path.join(dir, `multi-${i}.json`);
        const child = spawnSync(process.env.LAYOUT_QUALITY_BINARY, [...args, '-o', path.join(dir, `multi-${i}.svg`),
          '--layout-diagnostics-json', report], {encoding: 'utf8', timeout: 30000});
        assert.equal(child.status, 0, child.stderr);
        states.push(JSON.parse(fs.readFileSync(report)));
      }
      assert.deepEqual(states[0], states[1]);
      assert.equal(compare(states[0], states[1], multi.budgets).status, 'passed');
    } finally { fs.rmSync(dir, {recursive: true, force: true}); }
  });
