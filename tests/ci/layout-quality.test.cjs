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
function updateRawSummary(report) {
  for (const metric of ['length', 'bends'])
    report.summary.raw_polyline[metric] = report.edges.reduce((sum, edge) => sum + edge.raw_polyline[metric], 0);
}

test('independent crossing, overlap and clearance degradations fail with stable relationships', () => {
  for (const [metric, mutate, category] of [
    ['crossings', r => r.summary.visible_straight_sections.crossings++, 'crossing'],
    ['overlap', r => r.summary.visible_straight_sections.independent_overlap_length++, 'independent_overlap'],
    ['border', r => { r.summary.visible_border_minimum_deficit++; r.edges[0].visible_border.minimum_deficit++; }, 'visible_border_clearance'],
  ]) {
    const before = baseline();
    const after = baseline();
    mutate(after);
    after.findings.push({category, edge: after.edges[0].id, peer: after.edges[1].id,
      segment: 0, peer_segment: 0, location: {point: [0, 0], interval: [0, 1], coordinates: [0, 0], horizontal: true, visible_gap: -1}});
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
  after.edges[0].visible_border.minimum_deficit += 0.25;
  after.canvas.width *= 2;
  after.edges[0].raw_polyline.length *= 10;
  updateRawSummary(after);
  assert.equal(compare(before, after, budgets).status, 'passed');
  after.edges[0].raw_polyline.bends++;
  updateRawSummary(after);
  assert.equal(compare(before, after, budgets).status, 'failed');
  const relative = structuredClone(budgets);
  relative.length = {mode: 'gate', absolute: 0, relative: 0.1};
  const slight = baseline();
  slight.edges[0].raw_polyline.length *= 1.1;
  updateRawSummary(slight);
  assert.equal(compare(before, slight, relative).status, 'passed');
  slight.edges[0].raw_polyline.length += 1;
  updateRawSummary(slight);
  assert.equal(compare(before, slight, relative).status, 'failed');
});

test('route summary and edge totals must agree on both comparison sides', () => {
  for (const mutate of [
    r => r.summary.routed_paths++,
    r => r.summary.raw_polyline.bends++,
    r => r.summary.raw_polyline.length++,
    r => r.edges[0].raw_polyline.bends++,
    r => r.edges[0].raw_polyline.length++,
    r => r.summary.visible_border_minimum_deficit++,
    r => r.edges[0].visible_border.minimum_deficit++,
  ]) {
    const report = baseline();
    mutate(report);
    assert.throws(() => compare(baseline(), report, budgets), /differs from summary/);
    assert.throws(() => compare(report, baseline(), budgets), /differs from summary/);
  }
});

test('completeness flags require literal true on both comparison sides', () => {
  for (const key of ['pair_metrics_complete', 'findings_complete']) {
    for (const value of [undefined, null, false, 'false', 'true', 1, {}, []]) {
      const report = baseline();
      report.limits[key] = value;
      assert.throws(() => compare(baseline(), report, budgets), /incomplete or unsupported/);
      assert.throws(() => compare(report, baseline(), budgets), /incomplete or unsupported/);
    }
  }
});

test('border increases on multiple edges cannot hide behind an understated summary', () => {
  const report = baseline();
  report.edges[0].visible_border.minimum_deficit += 0.25;
  report.edges[1].visible_border.minimum_deficit += 0.25;
  report.summary.visible_border_minimum_deficit += 0.25;
  assert.throws(() => compare(baseline(), report, budgets), /non-comparable border/);
  report.summary.visible_border_minimum_deficit += 0.25;
  const result = compare(baseline(), report, budgets);
  assert.equal(result.status, 'failed');
  assert.deepEqual(result.changes.filter(c => c.failed).map(c => c.metric), ['border']);
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
  for (const edge of report.edges) {
    edge.id = mapping.get(edge.id);
    edge.identity = JSON.parse(edge.id);
  }
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

test('edge ID must encode the structured identity on both comparison sides', () => {
  for (const mutate of [
    edge => { edge.identity.source.id = 'different'; },
    edge => { delete edge.identity; },
    edge => { edge.identity = null; },
    edge => { edge.id = '!invalid-json'; },
  ]) {
    const report = baseline();
    mutate(report.edges[0]);
    assert.throws(() => compare(baseline(), report, budgets), /non-comparable/);
    assert.throws(() => compare(report, baseline(), budgets), /non-comparable/);
  }
});

test('canvas dimensions reject coercible and invalid numbers on both comparison sides', () => {
  for (const key of ['width', 'height']) {
    for (const value of [undefined, null, '100', false, [], -1, Infinity, NaN]) {
      const report = baseline();
      report.canvas[key] = value;
      assert.throws(() => compare(baseline(), report, budgets), /invalid canvas dimension/);
      assert.throws(() => compare(report, baseline(), budgets), /invalid canvas dimension/);
    }
  }
});

test('pair regressions cannot be offset by another pair, and reversed pair order is equivalent', () => {
  for (const [metric, category, amount] of [
    ['crossings', 'crossing', 1], ['overlap', 'independent_overlap', 100],
    ['parallel', 'parallel_proximity', 800],
  ]) {
    const before = baseline();
    const [a, b, c] = before.edges.map(e => e.id);
    before.findings = [{category, edge: a, peer: b, segment: 0, peer_segment: 0, location: {point: [0, 0], interval: [0, 100], coordinates: [0, 8], horizontal: true, visible_gap: 6}}];
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

test('pair findings reject missing locations and coerced numeric fields', () => {
  const before = JSON.parse(fs.readFileSync(path.join(__dirname, '../layout-quality/large.json'))).report;
  for (const category of ['crossing', 'parallel_proximity']) {
    const mutations = [f => { delete f.location; }, f => { delete f.segment; },
      f => { delete f.peer_segment; }, f => { f.segment = '0'; },
      f => { f.peer_segment = -1; }];
    for (const field of category === 'crossing' ? ['point'] : ['interval', 'coordinates']) {
      mutations.push(f => { f.location[field] = f.location[field].map(String); });
      mutations.push(f => { f.location[field] = [0]; });
    }
    for (const mutate of mutations) {
      const after = structuredClone(before);
      mutate(after.findings.find(f => f.category === category));
      assert.throws(() => compare(before, after, budgets), /invalid pair finding/);
      assert.throws(() => compare(after, before, budgets), /invalid pair finding/);
    }
  }
});

test('summaries bound large errors and relationship identities', () => {
  const text = markdown([{fixture: 'failure', status: 'non-comparable', error: 'x'.repeat(1000000)}]);
  assert.ok(text.length < 3000);
  assert.match(text, /truncated/);
  const changes = Array.from({length: 20}, () => ({metric: 'bends', edge: 'x'.repeat(100000),
    findings: [], failed: true, before: 0, after: 3, delta: 3, allowance: 2}));
  assert.ok(markdown([{fixture: 'large', status: 'failed', changes}]).length < 61000);
});

test('per-edge evidence is limited to the affected metric', () => {
  const before = JSON.parse(fs.readFileSync(path.join(__dirname, '../layout-quality/large.json'))).report;
  const after = structuredClone(before);
  const id = after.findings.find(f => f.category === 'crossing').edge;
  const edge = after.edges.find(e => e.id === id);
  edge.raw_polyline.bends += 3;
  edge.raw_polyline.length++;
  updateRawSummary(after);
  edge.visible_border.minimum_deficit++;
  after.summary.visible_border_minimum_deficit++;
  const result = compare(before, after, budgets);
  for (const change of result.changes.filter(c => c.edge === id)) {
    if (change.metric === 'edge_border')
      assert.ok(change.findings.every(f => f.category === 'visible_border_clearance' && f.edge === id));
    else assert.deepEqual(change.findings, []);
  }
});

test('reserved output names and invalid manifests fail before any output is written', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-manifest-'));
  try {
    const manifestPath = path.join(dir, 'custom.json');
    const output = path.join(dir, 'out');
    const fixture = manifest.fixtures[0];
    for (const fixtures of [[], [{...fixture, id: 'custom'}], [{...fixture, id: 'results'}],
      [{...fixture, args: null}], [{...fixture, inputs: []}], [{...fixture, budgets: {...budgets, bends: null}}]]) {
      const source = JSON.stringify({...manifest, fixtures});
      fs.writeFileSync(manifestPath, source);
      for (const update of [false, true]) {
        assert.throws(() => run({manifestPath, binary: 'must-not-run', output, update}));
        assert.equal(fs.readFileSync(manifestPath, 'utf8'), source);
        assert.equal(fs.existsSync(output), false);
      }
    }
    const empty = JSON.parse(fs.readFileSync(path.join(__dirname, '../layout-quality/accounts.json'))).report;
    empty.edges = [];
    empty.findings = [];
    empty.summary = {routed_paths: 0, raw_polyline: {length: 0, bends: 0}, visible_border_minimum_deficit: 0,
      visible_straight_sections: {crossings: 0, independent_overlap_length: 0, parallel_deficit: 0, shared_trunk_length: 0}};
    assert.equal(compare(empty, empty, budgets).status, 'passed');
    assert.throws(() => compare(empty, empty, {...budgets, bends: null}), /invalid budget: bends/);
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('duplicate fixture IDs are rejected without output or baseline writes', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-duplicate-'));
  try {
    const manifestPath = path.join(dir, 'manifest.json');
    const fixture = manifest.fixtures[0];
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture, {...fixture, args: ['different']}]}));
    const baselinePath = path.join(dir, `${fixture.id}.json`);
    fs.writeFileSync(baselinePath, 'preserved');
    const output = path.join(dir, 'out');
    for (const update of [false, true]) {
      assert.throws(() => run({manifestPath, binary: 'must-not-run', output, update}), /duplicate fixture IDs/);
      assert.equal(fs.readFileSync(baselinePath, 'utf8'), 'preserved');
      assert.equal(fs.existsSync(output), false);
    }
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('successful renderer without fresh regular diagnostics cannot check or update a baseline', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-stale-'));
  try {
    const fake = path.join(dir, 'fake.cjs');
    const fixture = {...manifest.fixtures[2], args: [fake]};
    fs.mkdirSync(path.join(dir, 'baselines'));
    const manifestPath = path.join(dir, 'baselines', 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    const options = {manifestPath, binary: process.execPath, output: path.join(dir, 'out')};
    const source = path.join(__dirname, '../layout-quality/fan-in.json');
    fs.writeFileSync(fake, `const fs = require('node:fs'); fs.writeFileSync(process.argv[process.argv.indexOf('-o') + 1], '<svg/>'); fs.writeFileSync(process.argv.at(-1), JSON.stringify(JSON.parse(fs.readFileSync(${JSON.stringify(source)})).report));`);
    assert.equal(run({...options, update: true}), true);
    const saved = fs.readFileSync(path.join(dir, 'baselines', 'fan-in.json'), 'utf8');
    for (const update of [false, true]) {
      fs.writeFileSync(path.join(options.output, 'fan-in.svg'), '<svg>stale</svg>');
      fs.writeFileSync(fake, `const fs = require('node:fs'); fs.writeFileSync(process.argv.at(-1), JSON.stringify(JSON.parse(fs.readFileSync(${JSON.stringify(source)})).report));`);
      assert.equal(run({...options, update}), false);
      assert.equal(fs.existsSync(path.join(options.output, 'fan-in.svg')), false);
      assert.equal(fs.readFileSync(path.join(dir, 'baselines', 'fan-in.json'), 'utf8'), saved);
    }
    for (const update of [false, true]) {
      fs.writeFileSync(path.join(options.output, 'fan-in.json'), JSON.stringify(baseline()));
      fs.writeFileSync(fake, '');
      assert.equal(run({...options, update}), false);
      assert.equal(fs.readFileSync(path.join(dir, 'baselines', 'fan-in.json'), 'utf8'), saved);
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
  after.findings.push({category: 'crossing', edge: after.edges[0].id, peer: after.edges[1].id, segment: 0, peer_segment: 0, location: {point: [0, 0]}});
  const text = markdown([{fixture: 'bad', ...compare(before, after, budgets)},
    {fixture: 'good', ...compare(after, before, budgets)}]);
  assert.match(text, /bad: failed/);
  assert.match(text, /good: passed/);
});

test('overlapping output directories are rejected before touching baselines or invoking the renderer', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-overlap-'));
  try {
    const baselineDir = path.join(dir, 'baselines');
    fs.mkdirSync(baselineDir);
    const baselinePath = path.join(baselineDir, 'fan-in.json');
    const saved = fs.readFileSync(path.join(__dirname, '../layout-quality/fan-in.json'));
    fs.writeFileSync(baselinePath, saved);
    const marker = path.join(dir, 'invoked');
    const fake = path.join(dir, 'fake.cjs');
    fs.writeFileSync(fake, `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'invoked'); process.exit(1);`);
    const manifestPath = path.join(baselineDir, 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [{...manifest.fixtures[2], args: [fake]}]}));
    const alias = path.join(dir, 'alias');
    fs.symlinkSync(baselineDir, alias, 'dir');
    for (const output of [baselineDir, path.join(baselineDir, '..', 'baselines'), dir,
      path.join(baselineDir, 'new', 'output'), alias, path.join(alias, 'new', 'output')]) {
      for (const update of [false, true]) {
        assert.throws(() => run({manifestPath, binary: process.execPath, output, update}), /must not overlap/);
        assert.deepEqual(fs.readFileSync(baselinePath), saved);
        assert.equal(fs.existsSync(marker), false);
        assert.equal(fs.existsSync(path.join(baselineDir, 'new')), false);
      }
    }
  } finally { fs.rmSync(dir, {recursive: true, force: true}); }
});

test('runner identifies missing and stale baselines and preserves full artifacts', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'layout-quality-'));
  try {
    const fake = path.join(dir, 'fake.cjs');
    fs.writeFileSync(fake, `const fs = require('node:fs'); fs.writeFileSync(process.argv[process.argv.indexOf('-o') + 1], '<svg/>'); const r = JSON.parse(fs.readFileSync(${JSON.stringify(path.join(__dirname, '../layout-quality/fan-in.json'))})); fs.writeFileSync(process.argv.at(-1), JSON.stringify(r.report));`);
    const fixture = {...manifest.fixtures[2], args: [fake]};
    fs.mkdirSync(path.join(dir, 'baselines'));
    const manifestPath = path.join(dir, 'baselines', 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    const options = {manifestPath, binary: process.execPath, output: path.join(dir, 'out')};
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /missing baseline/);
    assert.equal(run({...options, update: true}), true);
    assert.equal(run(options), true);
    fs.appendFileSync(fake, '\nconst report = JSON.parse(fs.readFileSync(process.argv.at(-1))); report.summary.visible_straight_sections.crossings++; report.findings.push({category: "crossing", edge: report.edges[0].id, peer: report.edges[1].id, segment: 0, peer_segment: 0, location: {point: [0, 0]}}); fs.writeFileSync(process.argv.at(-1), JSON.stringify(report));');
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /FAIL: crossings/);
    const originalBaseline = fs.readFileSync(path.join(dir, 'baselines', 'fan-in.json'), 'utf8');
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture,
      {...fixture, id: 'broken', inputs: [path.join(dir, 'missing.json')]}]}));
    assert.equal(run({...options, update: true}), false);
    assert.equal(fs.readFileSync(path.join(dir, 'baselines', 'fan-in.json'), 'utf8'), originalBaseline);
    fixture.budgets = {...budgets, bends: {mode: 'gate', absolute: 3, relative: 0}};
    fs.writeFileSync(manifestPath, JSON.stringify({...manifest, fixtures: [fixture]}));
    assert.equal(run(options), false);
    assert.match(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8'), /stale baseline/);
    assert.ok(fs.existsSync(path.join(options.output, 'results.json')));
    const protectedFile = path.join(dir, 'protected.txt');
    fs.writeFileSync(protectedFile, 'preserved');
    for (const filename of ['fan-in.log', 'results.json', 'summary.md']) {
      fs.unlinkSync(path.join(options.output, filename));
      fs.symlinkSync(protectedFile, path.join(options.output, filename));
    }
    fs.writeFileSync(fake, "require('node:fs').writeSync(2, 'x'.repeat(100000)); process.exit(1);");
    assert.equal(run(options), false);
    assert.ok(fs.readFileSync(path.join(options.output, 'summary.md'), 'utf8').length < 3000);
    assert.equal(fs.readFileSync(path.join(options.output, 'fan-in.log'), 'utf8').length, 100000);
    assert.equal(fs.readFileSync(protectedFile, 'utf8'), 'preserved');
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
