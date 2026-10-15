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
const artifact = (file, value) => {
  fs.rmSync(file, {force: true});
  fs.writeFileSync(file, value, {flag: 'wx'});
};
const hash = data => crypto.createHash('sha256').update(data).digest('hex');
const utf8Order = (a, b) => Buffer.compare(Buffer.from(a, 'utf8'), Buffer.from(b, 'utf8'));
const bounded = (value, limit = 2000) => {
  const suffix = '\n[truncated; see fixture log or results.json]';
  return value.length <= limit ? value : value.slice(0, limit - suffix.length) + suffix;
};

function validateBudgets(budgets) {
  for (const metric of [...Object.keys(metrics), ...Object.keys(edgeMetrics)]) {
    const budget = budgets?.[metric];
    if (!budget || !['gate', 'info'].includes(budget.mode) ||
        !Number.isFinite(budget.absolute) || budget.absolute < 0 ||
        !Number.isFinite(budget.relative) || budget.relative < 0)
      throw new Error(`missing or invalid budget: ${metric}`);
  }
}

function resolvedDirectory(directory) {
  const absolute = path.resolve(directory);
  try { return fs.realpathSync(absolute); }
  catch (error) {
    if (error.code !== 'ENOENT') throw error;
    return path.join(resolvedDirectory(path.dirname(absolute)), path.basename(absolute));
  }
}

function containsDirectory(parent, child) {
  const relative = path.relative(parent, child);
  return relative === '' || (relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative));
}

function validate(report) {
  if (!Array.isArray(report.findings)) throw new Error('non-comparable missing or invalid findings array');
  if (report.schema_version !== 1 || report.category !== 'layout' || report.scope !== 'whole_diagram')
    throw new Error('incompatible diagnostics schema/category/scope');
  if (report.limits?.pair_metrics_complete !== true || report.limits?.findings_complete !== true ||
      report.limits.unsupported_diagonal_segments !== 0)
    throw new Error('non-comparable incomplete or unsupported diagnostics');
  const ids = report.edges.map(e => e.id);
  if (ids.some(id => typeof id !== 'string') ||
      !isDeepStrictEqual(ids, [...ids].sort(utf8Order)) || new Set(ids).size !== ids.length)
    throw new Error('non-comparable edge ordering or duplicate stable identities');
  for (const edge of report.edges) {
    let identity;
    try { identity = JSON.parse(edge.id); }
    catch { throw new Error('non-comparable edge ID is not JSON'); }
    if (!isDeepStrictEqual(identity, edge.identity))
      throw new Error('non-comparable edge ID does not match structured identity');
  }
  for (const dimension of [report.canvas?.width, report.canvas?.height]) {
    if (!Number.isFinite(dimension) || dimension < 0)
      throw new Error('invalid canvas dimension');
  }
  for (const value of [report.summary?.routed_paths, report.summary?.raw_polyline?.length,
    report.summary?.raw_polyline?.bends, report.summary?.visible_straight_sections?.shared_trunk_length,
    ...Object.values(metrics).map(f => f(report)),
    ...report.edges.flatMap(e => Object.values(edgeMetrics).map(f => f(e)))]) {
    if (!Number.isFinite(value) || value < 0) throw new Error('invalid numeric measurement');
  }
  for (const metric of ['crossings', 'overlap', 'parallel']) {
    const total = [...pairs(report, categories[metric]).values()].reduce((sum, value) => sum + value, 0);
    if (!Number.isFinite(total) || Math.abs(total - metrics[metric](report)) > 1e-6)
      throw new Error(`non-comparable ${metric}: findings total ${total} differs from summary ${metrics[metric](report)}`);
  }
  if (report.summary.routed_paths !== report.edges.length)
    throw new Error('non-comparable routed_paths: edge count differs from summary');
  const borderTotal = report.edges.reduce((sum, edge) => sum + edge.visible_border.minimum_deficit, 0);
  if (!Number.isFinite(borderTotal) || Math.abs(borderTotal - metrics.border(report)) > 1e-6)
    throw new Error(`non-comparable border: edge total ${borderTotal} differs from summary ${metrics.border(report)}`);
  for (const metric of ['length', 'bends']) {
    const total = report.edges.reduce((sum, edge) => sum + edge.raw_polyline[metric], 0);
    if (!Number.isFinite(total) || Math.abs(total - report.summary.raw_polyline[metric]) > 1e-6)
      throw new Error(`non-comparable raw ${metric}: edge total ${total} differs from summary ${report.summary.raw_polyline[metric]}`);
  }
}

function pairs(report, category) {
  const values = new Map();
  const ids = new Set(report.edges.map(e => e.id));
  for (const finding of report.findings.filter(f => f.category === category)) {
    if (!ids.has(finding.edge) || !ids.has(finding.peer) || finding.edge === finding.peer)
      throw new Error('non-comparable finding relationship identities');
    const key = JSON.stringify([finding.edge, finding.peer].sort(utf8Order));
    const vector = value => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite);
    const location = finding.location;
    if (!Number.isInteger(finding.segment) || finding.segment < 0 ||
        !Number.isInteger(finding.peer_segment) || finding.peer_segment < 0 || !location ||
        (category === 'crossing' ? !vector(location.point) :
          !vector(location.interval) || location.interval[0] >= location.interval[1] ||
          !vector(location.coordinates) || typeof location.horizontal !== 'boolean' ||
          !Number.isFinite(location.visible_gap)))
      throw new Error('non-comparable invalid pair finding location or segment');
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
  validateBudgets(budgets);
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
    const delta = current - old;
    const allowance = budget.absolute + old * budget.relative;
    if (!Number.isFinite(allowance)) throw new Error(`budget allowance overflow: ${metric}`);
    const failed = budget.mode === 'gate' && delta > allowance + 1e-6;
    if (delta !== 0) changes.push({metric, edge, peer, before: old, after: current, delta,
      allowance, mode: budget.mode, failed,
      findings: after.findings.filter(f => peer ? f.category === categories[metric] &&
        ((f.edge === edge && f.peer === peer) || (f.edge === peer && f.peer === edge)) :
        edge ? metric === 'edge_border' && f.category === 'visible_border_clearance' && f.edge === edge :
          f.category === categories[metric])});
  };
  for (const [name, measure] of Object.entries(metrics)) check(name, measure(before), measure(after));
  for (const metric of ['crossings', 'overlap', 'parallel']) {
    const old = pairs(before, categories[metric]);
    const current = pairs(after, categories[metric]);
    for (const key of [...new Set([...old.keys(), ...current.keys()])].sort(utf8Order))
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
    if (result.error) lines.push(bounded(result.error), '');
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
  return bounded(lines.join('\n'), 60000);
}

function run({binary, output, update = false, manifestPath = 'tests/layout-quality/manifest.json'}) {
  const manifest = read(manifestPath);
  if (manifest.version !== 1 || manifest.renderer_contract !== 'planorama-final-layout-v1')
    throw new Error('incompatible manifest/renderer contract');
  if (!Array.isArray(manifest.fixtures) || manifest.fixtures.length === 0)
    throw new Error('manifest must contain fixtures');
  const ids = manifest.fixtures.map(fixture => fixture.id);
  if (ids.some(id => typeof id !== 'string' || !/^[a-z0-9][a-z0-9-]*$/.test(id)) || new Set(ids).size !== ids.length)
    throw new Error('invalid or duplicate fixture IDs');
  const baselineDirectory = fs.realpathSync(path.dirname(manifestPath));
  const manifestFile = fs.realpathSync(manifestPath);
  for (const fixture of manifest.fixtures) {
    const baselineFile = path.join(baselineDirectory, `${fixture.id}.json`);
    if (baselineFile === manifestFile || fixture.id === 'results' ||
        (fs.existsSync(baselineFile) && fs.realpathSync(baselineFile) === manifestFile))
      throw new Error(`reserved fixture path: ${fixture.id}`);
    if (!Array.isArray(fixture.args) || !fixture.args.every(arg => typeof arg === 'string') ||
        !Array.isArray(fixture.inputs) || fixture.inputs.length === 0 ||
        !fixture.inputs.every(input => typeof input === 'string'))
      throw new Error(`invalid fixture arguments or inputs: ${fixture.id}`);
    validateBudgets(fixture.budgets);
  }
  const outputDirectory = resolvedDirectory(output);
  if (containsDirectory(baselineDirectory, outputDirectory) || containsDirectory(outputDirectory, baselineDirectory))
    throw new Error('output directory must not overlap the baseline directory');
  fs.mkdirSync(output, {recursive: true});
  for (const filename of ['results.json', 'summary.md'])
    fs.rmSync(path.join(output, filename), {force: true});
  const results = [];
  const pending = [];
  for (const fixture of manifest.fixtures) {
    try {
      const inputs = fixture.inputs.map(file => ({path: file, sha256: hash(fs.readFileSync(file))}));
      const contract = {version: manifest.version, renderer: manifest.renderer_contract,
        fixture: fixture.id, args: fixture.args, inputs, budgets: fixture.budgets};
      const baselinePath = path.join(path.dirname(manifestPath), `${fixture.id}.json`);
      const reportPath = path.join(output, `${fixture.id}.json`);
      const svgPath = path.join(output, `${fixture.id}.svg`);
      fs.rmSync(reportPath, {force: true});
      fs.rmSync(svgPath, {force: true});
      const child = spawnSync(binary, [...fixture.args, '-o', svgPath,
        '--layout-diagnostics-json', reportPath], {encoding: 'utf8', timeout: 180000});
      artifact(path.join(output, `${fixture.id}.log`), (child.stdout || '') + (child.stderr || ''));
      if (child.error || child.status !== 0)
        throw new Error(`render failed; see ${fixture.id}.log: ${bounded(String(child.error || child.stderr))}`);
      if (!fs.lstatSync(reportPath).isFile()) throw new Error('renderer did not create a regular diagnostics file');
      if (!fs.lstatSync(svgPath).isFile()) throw new Error('renderer did not create a regular SVG file');
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
  artifact(path.join(output, 'results.json'), stable(results));
  artifact(path.join(output, 'summary.md'), markdown(results));
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
