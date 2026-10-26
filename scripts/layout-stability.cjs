const fs = require('node:fs');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const {createHash} = require('node:crypto');

const canonical = value => JSON.stringify(value, function(key, item) {
  return item && typeof item === 'object' && !Array.isArray(item)
    ? Object.fromEntries(Object.keys(item).sort().map(key => [key, item[key]])) : item;
});
const order = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const identityValid = id => id && typeof id.address === 'string' &&
  (id.state === null || typeof id.state === 'string') && ['managed', 'data'].includes(id.mode) &&
  (id.deposed === null || typeof id.deposed === 'string');
const frameValid = b => b && ['x','y','width','height'].every(k => Number.isFinite(b[k])) && b.width > 0 && b.height > 0;
const vector = (dx, dy) => ({dx, dy, distance: Math.hypot(dx, dy)});
const aggregate = values => ({count: values.length, mean: values.length ? values.reduce((sum, v) => sum + v, 0) / values.length : null, maximum: values.length ? values.reduce((maximum, value) => Math.max(maximum,value),0) : null});

function compare(before, after, context = {}) {
  if (context.affected !== undefined && (!Array.isArray(context.affected) || !context.affected.every(identityValid))) throw new Error('Invalid declared affected identities');
  const unmatchable = [];
  const index = (report, side) => {
    if (report.schema_version !== 1 || report.category !== 'layout' || report.resource_geometry?.schema_version !== 1 || !Array.isArray(report.resource_geometry.records)) throw new Error('Unsupported final-layout resource geometry contract');
    const rows = new Map();
    for (const record of report.resource_geometry.records) {
      if (!identityValid(record.identity)) {
        unmatchable.push({side, identity: record.identity || null, reason:'invalid or unavailable structured identity'});
        continue;
      }
      const key = canonical(record.identity);
      if (!rows.has(key)) rows.set(key, []);
      rows.get(key).push(record);
    }
    return rows;
  };
  const first = index(before, 'before'), second = index(after, 'after');
  const affected = new Set((context.affected || []).map(canonical));
  const matched = [], added = [], removed = [], unmeasured = [];
  for (const key of [...new Set([...first.keys(), ...second.keys()])].sort(order)) {
    const oldRows = first.get(key) || [], newRows = second.get(key) || [];
    const identity = JSON.parse(key);
    if (oldRows.length > 1 || newRows.length > 1) {
      unmatchable.push({identity, reason:'duplicate identity', before_count:oldRows.length, after_count:newRows.length});
      continue;
    }
    if (!oldRows.length) { added.push(identity); continue; }
    if (!newRows.length) { removed.push(identity); continue; }
    const old = oldRows[0], next = newRows[0];
    if (!frameValid(old.bounds) || !frameValid(next.bounds)) {
      unmeasured.push({identity, before:old.representation, after:next.representation,
        before_reason:old.unavailable_reason || (!frameValid(old.bounds) ? 'invalid bounds' : null),
        after_reason:next.unavailable_reason || (!frameValid(next.bounds) ? 'invalid bounds' : null)});
      continue;
    }
    let local = null;
    const validScope = scope => scope?.identity && Array.isArray(scope.origin) && scope.origin.length === 2 && scope.origin.every(Number.isFinite);
    for (const scope of old.scopes || []) {
      if (!validScope(scope)) continue;
      const candidates = (next.scopes || []).filter(candidate => validScope(candidate) && canonical(candidate.identity) === canonical(scope.identity));
      if (candidates.length !== 1) continue;
      const other = candidates[0];
      local = {scope:scope.identity, ...vector((next.bounds.x - other.origin[0]) - (old.bounds.x - scope.origin[0]),
        (next.bounds.y - other.origin[1]) - (old.bounds.y - scope.origin[1]))};
      break;
    }
    matched.push({identity, before:old.bounds, after:next.bounds,
      representation:{before:old.representation, after:next.representation},
      whole_diagram:vector(next.bounds.x - old.bounds.x, next.bounds.y - old.bounds.y),
      relative_to_shared_scope:local,
      size:{width:next.bounds.width - old.bounds.width, height:next.bounds.height - old.bounds.height,
        area:next.bounds.width * next.bounds.height - old.bounds.width * old.bounds.height},
      declared_unrelated: context.intentional_restructure && !affected.size ? null : !affected.has(key)});
  }
  const distances = rows => ({whole_diagram:aggregate(rows.map(r => r.whole_diagram.distance)),
    relative_to_shared_scope:aggregate(rows.filter(r => r.relative_to_shared_scope).map(r => r.relative_to_shared_scope.distance))});
  const firstMove = matched[0]?.whole_diagram;
  const translation = firstMove && matched.every(r => r.whole_diagram.dx === firstMove.dx && r.whole_diagram.dy === firstMove.dy)
    ? {dx:firstMove.dx, dy:firstMove.dy} : null;
  return {schema_version:1, category:'layout_stability', units:'SVG user units (px)', context,
    summary:{matched:matched.length, added:added.length, removed:removed.length, unmeasured:unmeasured.length,
      unmatchable:unmatchable.length, displacement:distances(matched), unrelated_displacement:distances(matched.filter(r => r.declared_unrelated)),
      width_change:aggregate(matched.map(r => Math.abs(r.size.width))), height_change:aggregate(matched.map(r => Math.abs(r.size.height))),
      uniform_translation_of_measured_resources:translation},
    largest_movements:[...matched].sort((a,b) => b.whole_diagram.distance - a.whole_diagram.distance || order(canonical(a.identity), canonical(b.identity))).slice(0,10),
    largest_local_movements:matched.filter(r => r.relative_to_shared_scope).sort((a,b) => b.relative_to_shared_scope.distance - a.relative_to_shared_scope.distance || order(canonical(a.identity), canonical(b.identity))).slice(0,10),
    matched, added, removed, unmeasured, unmatchable:unmatchable.sort((a,b) => order(canonical(a),canonical(b))),
    relationships:{compared:false, reason:'Routing geometry is not part of this initial resource displacement metric.'}};
}

function patch(input, patches = []) {
  const result = structuredClone(input);
  for (const change of patches) {
    if (!['add','replace','remove'].includes(change.op) || !Array.isArray(change.path) || !change.path.length || change.path.some(k => ['__proto__','prototype','constructor'].includes(k))) throw new Error('Invalid fixture patch');
    let parent = result;
    for (const key of change.path.slice(0,-1)) {
      if (!Object.hasOwn(parent, key)) throw new Error('Fixture patch parent missing');
      parent = parent[key];
    }
    const key = change.path.at(-1);
    if (change.op !== 'add' && !Object.hasOwn(parent, key)) throw new Error('Fixture patch target missing');
    if (Array.isArray(parent)) {
      const index = key === '-' ? parent.length : key;
      if (!Number.isInteger(index) || index < 0 || index > parent.length) throw new Error('Invalid fixture array index');
      if (change.op === 'remove') parent.splice(index,1);
      else if (change.op === 'add') parent.splice(index,0,structuredClone(change.value));
      else parent[index] = structuredClone(change.value);
    } else if (change.op === 'remove') delete parent[key];
    else parent[key] = structuredClone(change.value);
  }
  return result;
}

function run(binary, manifestFile, directory) {
  const manifest = manifestFile.endsWith('.cjs') ? require(path.resolve(manifestFile)) : JSON.parse(fs.readFileSync(manifestFile,'utf8'));
  if (manifest.schema_version !== 1 || !Array.isArray(manifest.pairs)) throw new Error('Unsupported fixture manifest');
  const results = [];
  const ids = new Set();
  fs.mkdirSync(directory,{recursive:true});
  for (const pair of manifest.pairs) {
    if (!/^[a-z0-9-]+$/.test(pair.id) || ids.has(pair.id)) throw new Error('Invalid or duplicate pair id');
    ids.add(pair.id);
    const folder = path.join(directory,pair.id);
    fs.mkdirSync(folder,{recursive:true});
    const generate = (side, config) => {
      const args = [];
      const inputs = [];
      for (const [index, state] of config.states.entries()) {
        const input = JSON.stringify(patch(JSON.parse(fs.readFileSync(state.input,'utf8')),state.patches),null,2) + '\n';
        const file = path.join(folder,`${side}-${index}.plan.json`);
        fs.writeFileSync(file,input);
        inputs.push({state:state.name, sha256:createHash('sha256').update(input).digest('hex')});
        if (state.name === null && config.states.length === 1) args.push(file);
        else args.push('--state',`${state.name}=${file}`);
      }
      args.push(...(config.args || []));
      const diagram = path.join(folder,`${side}.svg`), reportFile = path.join(folder,`${side}.json`);
      execFileSync(binary,[...args,'-o',diagram,'--layout-diagnostics-json',reportFile],{stdio:['ignore','pipe','pipe'],maxBuffer:64*1024*1024});
      const bytes = fs.readFileSync(diagram);
      const plain = execFileSync(binary,[...args,'-o','-'],{maxBuffer:64*1024*1024});
      if (!bytes.equals(plain)) throw new Error(`${pair.id}/${side}: diagnostics changed diagram bytes`);
      return {report:JSON.parse(fs.readFileSync(reportFile,'utf8')), inputs, svg_sha256:createHash('sha256').update(bytes).digest('hex')};
    };
    const before = generate('before',pair.before), after = generate('after',pair.after);
    const context = {id:pair.id, description:pair.description, intentional_restructure:pair.intentional_restructure === true,
      affected:pair.affected || [], before:{args:pair.before.args || [],inputs:before.inputs,svg_sha256:before.svg_sha256},
      after:{args:pair.after.args || [],inputs:after.inputs,svg_sha256:after.svg_sha256}};
    const report = compare(before.report,after.report,context);
    if (pair.control && (report.matched.some(r => r.whole_diagram.distance || r.size.width || r.size.height || r.relative_to_shared_scope?.distance) || report.added.length || report.removed.length || report.unmatchable.length || before.svg_sha256 !== after.svg_sha256)) throw new Error('Identical control was not zero');
    fs.writeFileSync(path.join(folder,'comparison.json'),JSON.stringify(report,null,2)+'\n');
    results.push({id:pair.id, intentional_restructure:context.intentional_restructure, summary:report.summary});
  }
  const result = {schema_version:1,category:'layout_stability_suite',pairs:results};
  fs.writeFileSync(path.join(directory,'summary.json'),JSON.stringify(result,null,2)+'\n');
  return result;
}

if (require.main === module) {
  try {
    const args = process.argv.slice(2);
    if (args.length !== 3) throw new Error('Usage: node scripts/layout-stability.cjs BINARY MANIFEST OUTPUT_DIRECTORY');
    const result = run(path.resolve(args[0]),args[1],args[2]);
    for (const pair of result.pairs) console.log(`${pair.id}: matched=${pair.summary.matched}, max movement=${pair.summary.displacement.whole_diagram.maximum}, added=${pair.summary.added}, removed=${pair.summary.removed}, unmeasured=${pair.summary.unmeasured}`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
module.exports = {compare,patch,run};
