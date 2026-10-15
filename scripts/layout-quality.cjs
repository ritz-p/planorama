const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {spawnSync} = require('node:child_process');
const {isDeepStrictEqual} = require('node:util');

const metrics = {
  crossings: r => r.summary.visible_straight_sections.crossings,
  overlap: r => r.summary.visible_straight_sections.independent_overlap_length,
  parallel: r => r.summary.visible_straight_sections.parallel_deficit,
  border: r => r.summary.visible_border_minimum_deficit,
  area: r => r.canvas.width * r.canvas.height,
};
const edgeMetrics = {
  bends: e => e.raw_polyline.bends,
  length: e => e.raw_polyline.length,
  edge_border: e => e.visible_border.minimum_deficit,
};
const categories = {crossings: 'crossing', overlap: 'independent_overlap',
  parallel: 'parallel_proximity', border: 'visible_border_clearance'};
const stable = value => JSON.stringify(value, null, 2) + '\n';
const read = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const write = (file, value) => fs.writeFileSync(file, stable(value));
const hash = data => crypto.createHash('sha256').update(data).digest('hex');

function validate(report) {
  if (!Array.isArray(report.findings)) throw new Error('non-comparable missing or invalid findings array');
  if (report.schema_version !== 1 || report.category !== 'layout' || report.scope !== 'whole_diagram')
    throw new Error('incompatible diagnostics schema/category/scope');
  if (!report.limits?.pair_metrics_complete || !report.limits?.findings_complete ||
      report.limits.unsupported_diagonal_segments !== 0)
    throw new Error('non-comparable incomplete or unsupported diagnostics');
  const ids = report.edges.map(e => e.id);
  if (!isDeepStrictEqual(ids, [...ids].sort()) || new Set(ids).size !== ids.length)
    throw new Error('non-comparable edge ordering or duplicate stable identities');
  for (const value of [...Object.values(metrics).map(f => f(report)),
    ...report.edges.flatMap(e => Object.values(edgeMetrics).map(f => f(e)))]) {
    if (!Number.isFinite(value) || value < 0) throw new Error('invalid numeric measurement');
  }
}

function pairs(report, category) {
  const values = new Map();
  const ids = new Set(report.edges.map(e => e.id));
  for (const finding of report.findings.filter(f => f.category === category)) {
    if (!ids.has(finding.edge) || !ids.has(finding.peer) || finding.edge === finding.peer)
      throw new Error('non-comparable finding relationship identities');
    const key = JSON.stringify([finding.edge, finding.peer].sort());
    let value = 1;
    if (category !== 'crossing') {
      const {interval, coordinates} = finding.location;
      value = interval[1] - interval[0];
      if (category === 'parallel_proximity')
        value *= report.policy.parallel_centerline_preference - Math.abs(coordinates[1] - coordinates[0]);
    }
    if (!Number.isFinite(value) || value <= 0) throw new Error('invalid pair finding measurement');
    values.set(key, (values.get(key) || 0) + value);
  }
  return values;
}

function compare(before, after, budgets) {
  validate(before);
  validate(after);
  for (const key of ['schema_version', 'category', 'scope', 'units', 'policy']) {
    if (!isDeepStrictEqual(before[key], after[key])) throw new Error(`incompatible report ${key}`);
  }
  if (!isDeepStrictEqual(before.edges.map(e => e.id), after.edges.map(e => e.id)) ||
      !isDeepStrictEqual(before.states.map(s => s.state), after.states.map(s => s.state)))
    throw new Error('stale baseline: stable edge/state identities changed');
  const changes = [];
  const check = (metric, old, current, edge, peer) => {
    const budget = budgets[metric];
    if (!budget || !['gate', 'info'].includes(budget.mode) ||
        !Number.isFinite(budget.absolute) || budget.absolute < 0 ||
        !Number.isFinite(budget.relative) || budget.relative < 0)
      throw new Error(`missing or invalid budget: ${metric}`);
    const delta = current - old;
    const allowance = budget.absolute + old * budget.relative;
    const failed = budget.mode === 'gate' && delta > allowance + 1e-6;
    if (delta !== 0) changes.push({metric, edge, peer, before: old, after: current, delta,
      allowance, mode: budget.mode, failed,
      findings: after.findings.filter(f => peer ? f.category === categories[metric] &&
        ((f.edge === edge && f.peer === peer) || (f.edge === peer && f.peer === edge)) :
        edge ? f.edge === edge || f.peer === edge : f.category === categories[metric])});
  };
  for (const [name, measure] of Object.entries(metrics)) check(name, measure(before), measure(after));
  for (const metric of ['crossings', 'overlap', 'parallel']) {
    const old = pairs(before, categories[metric]);
    const current = pairs(after, categories[metric]);
    for (const key of [...new Set([...old.keys(), ...current.keys()])].sort())
      check(metric, old.get(key) || 0, current.get(key) || 0, ...JSON.parse(key));
  }
  before.edges.forEach((edge, i) => {
    for (const [name, measure] of Object.entries(edgeMetrics))
      check(name, measure(edge), measure(after.edges[i]), edge.id);
  });
  return {status: changes.some(c => c.failed) ? 'failed' : 'passed', changes};
}

function markdown(results) {
  const lines = ['# Layout quality', ''];
  for (const result of results) {
    lines.push(`## ${result.fixture}: ${result.status}`, '');
    if (result.error) lines.push(result.error, '');
    const changes = [...(result.changes || [])].sort((a, b) => Number(b.failed) - Number(a.failed));
    for (const c of changes.slice(0, 20)) {
      lines.push(`- ${c.failed ? 'FAIL' : c.mode}: ${c.metric}: ${c.before} → ${c.after} (delta ${c.delta}, allowance ${c.allowance})`);
      if (c.edge) lines.push(`  - Relationship: \`${c.edge}\``);
      if (c.peer) lines.push(`  - Peer: \`${c.peer}\``);
      for (const f of c.findings.slice(0, 3))
        lines.push(`  - \`${f.edge}\`${f.peer ? ` / \`${f.peer}\`` : ''}`);
      if (c.findings.length > 3) lines.push(`  - ${c.findings.length - 3} more findings in results.json`);
    }
    if (changes.length > 20) lines.push(`- ${changes.length - 20} more metric changes in results.json`);
    lines.push('');
  }
  return lines.join('\n');
}

function run({binary, output, update = false, manifestPath = 'tests/layout-quality/manifest.json'}) {
  const manifest = read(manifestPath);
  if (manifest.version !== 1 || manifest.renderer_contract !== 'planorama-final-layout-v1')
    throw new Error('incompatible manifest/renderer contract');
  fs.mkdirSync(output, {recursive: true});
  const results = [];
  const pending = [];
  for (const fixture of manifest.fixtures) {
    try {
      const inputs = fixture.inputs.map(file => ({path: file, sha256: hash(fs.readFileSync(file))}));
      const contract = {version: manifest.version, renderer: manifest.renderer_contract,
        fixture: fixture.id, args: fixture.args, inputs, budgets: fixture.budgets};
      const baselinePath = path.join(path.dirname(manifestPath), `${fixture.id}.json`);
      const reportPath = path.join(output, `${fixture.id}.json`);
      fs.rmSync(reportPath, {force: true});
      const child = spawnSync(binary, [...fixture.args, '-o', path.join(output, `${fixture.id}.svg`),
        '--layout-diagnostics-json', reportPath], {encoding: 'utf8', timeout: 180000});
      fs.writeFileSync(path.join(output, `${fixture.id}.log`), (child.stdout || '') + (child.stderr || ''));
      if (child.error || child.status !== 0) throw new Error(`render failed: ${child.error || child.stderr}`);
      if (!fs.lstatSync(reportPath).isFile()) throw new Error('renderer did not create a regular diagnostics file');
      const report = read(reportPath);
      validate(report);
      if (update) {
        compare(report, report, fixture.budgets);
        pending.push([baselinePath, {contract, report}]);
        results.push({fixture: fixture.id, status: 'updated', changes: []});
      } else {
        if (!fs.existsSync(baselinePath)) throw new Error('missing baseline; explicit update required');
        const baseline = read(baselinePath);
        if (!isDeepStrictEqual(baseline.contract, contract))
          throw new Error('stale baseline: fixture bytes, CLI options, budgets or renderer contract changed');
        results.push({fixture: fixture.id, ...compare(baseline.report, report, fixture.budgets)});
      }
    } catch (error) {
      results.push({fixture: fixture.id, status: 'non-comparable', error: error.message});
    }
  }
  const ok = results.length > 0 && results.every(r => ['passed', 'updated'].includes(r.status));
  if (update && ok) for (const [file, value] of pending) write(file, value);
  write(path.join(output, 'results.json'), results);
  fs.writeFileSync(path.join(output, 'summary.md'), markdown(results));
  return ok;
}

if (require.main === module) {
  const args = process.argv.slice(2);
  if (!['check', 'update'].includes(args[0]) || args.length !== 3) {
    console.error('Usage: node scripts/layout-quality.cjs check|update BINARY OUTPUT_DIRECTORY');
    process.exitCode = 2;
  } else {
    try { process.exitCode = run({update: args[0] === 'update', binary: args[1], output: args[2]}) ? 0 : 1; }
    catch (error) { console.error(error.message); process.exitCode = 2; }
  }
}
module.exports = {compare, markdown, run};
