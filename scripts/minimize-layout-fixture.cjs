const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {execFileSync} = require('node:child_process');
const {isDeepStrictEqual} = require('node:util');
const {validate, pairs} = require('./layout-quality.cjs');

function requireShape(condition) {
  if (!condition) throw new Error('Expected a flat managed-create fixture with explicit providers and local resource references');
}
function onlyKeys(object, allowed) {
  return object && typeof object === 'object' && !Array.isArray(object) &&
    Object.keys(object).every(key => allowed.includes(key));
}
function references(expression, visit) {
  if (!expression || typeof expression !== 'object') return;
  for (const [key, value] of Object.entries(expression)) {
    if (key === 'references') {
      requireShape(Array.isArray(value));
      for (const ref of value) visit(ref);
    } else if (key !== 'constant_value') references(value, visit);
  }
}

function dependencies(plan) {
  requireShape(onlyKeys(plan, ['format_version','terraform_version','configuration','resource_changes']) &&
    onlyKeys(plan.configuration, ['provider_config','root_module']) &&
    onlyKeys(plan.configuration.root_module, ['resources']));
  const changes = plan.resource_changes;
  const {provider_config: providers, root_module: {resources}} = plan.configuration;
  requireShape(Array.isArray(changes) && Array.isArray(resources) && providers &&
    changes.length > 0 && resources.length === changes.length);
  const byAddress = new Map(changes.map(item => [item.address, item]));
  requireShape(byAddress.size === changes.length);
  references(providers, () => requireShape(false));
  const deps = new Map();
  for (const item of resources) {
    const change = byAddress.get(item.address);
    requireShape(onlyKeys(item, ['address','provider_config_key','expressions']) &&
      /^[a-zA-Z_][\w-]*\.[a-zA-Z_][\w-]*$/.test(item.address) && !deps.has(item.address) &&
      onlyKeys(change, ['address','type','name','mode','provider_name','change']) && change.mode === 'managed' &&
      item.address.split('.')[0] === change.type && (!('name' in change) || change.name === item.address.split('.')[1]) &&
      typeof change.provider_name === 'string' && Object.hasOwn(providers, item.provider_config_key) &&
      providers[item.provider_config_key].full_name === change.provider_name &&
      onlyKeys(change.change, ['actions','before','after','after_unknown','before_sensitive','after_sensitive']) &&
      isDeepStrictEqual(change.change.actions, ['create']));
    const targets = new Set();
    references(item.expressions, ref => {
      requireShape(typeof ref === 'string');
      const address = ref.split('.').slice(0, 2).join('.');
      requireShape(byAddress.has(address));
      targets.add(address);
    });
    deps.set(item.address, targets);
  }
  return deps;
}

function project(plan, kept) {
  const remaining = resource => kept.has(resource.address);
  return {...plan, resource_changes: plan.resource_changes.filter(remaining),
    configuration: {...plan.configuration, root_module: {resources: plan.configuration.root_module.resources.filter(remaining)}}};
}

function minimize(binary, input, index, output, budget) {
  const text = fs.readFileSync(input, 'utf8');
  const plan = JSON.parse(text);
  const tokens = /"(?:\\.|[^"\\])*"|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g;
  for (const [, literal] of text.matchAll(tokens)) {
    if (literal && (!Number.isSafeInteger(Number(literal)) || String(Number(literal)) !== literal))
      throw new Error('Only losslessly preserved integer literals are supported');
  }
  if (fs.existsSync(output)) throw new Error('Output already exists');
  const deps = dependencies(plan);
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'planorama-reduce-'));
  const inFile = path.join(scratch, 'input.json');
  const reportFile = path.join(scratch, 'diagnostics.json');
  const graphFile = path.join(scratch, 'architecture.json');
  const read = file => JSON.parse(fs.readFileSync(file, 'utf8'));
  function render(file, args) {
    execFileSync(binary, [file, ...args], {timeout: 30000, stdio: 'ignore'});
  }
  function report(file) {
    render(file, ['-o', path.join(scratch,'diagram.svg'), '--layout-diagnostics-json', reportFile]);
    const diagnostics = read(reportFile);
    validate(diagnostics);
    return diagnostics;
  }
  function graph(file) {
    render(file, ['--format','json','-o',graphFile]);
    return read(graphFile);
  }
  try {
    const original = graph(input);
    requireShape(original.schema_version === 1 && original.resources.length === deps.size &&
      original.resources.every(r => r.kind === 'resource' && deps.has(r.address)));
    const initial = report(input);
    const finding = initial.findings[index];
    if (!finding || !['crossing','independent_overlap','parallel_proximity'].includes(finding.category))
      throw new Error('Select a crossing, independent overlap or parallel proximity finding index');
    const pair = JSON.stringify([finding.edge, finding.peer].sort((a,b) => Buffer.compare(Buffer.from(a), Buffer.from(b))));
    const score = diagnostics => pairs(diagnostics, finding.category).get(pair) ?? 0;
    const threshold = score(initial);
    if (threshold <= 0) throw new Error('Missing original edge-pair measurement');
    function measure(kept) {
      fs.writeFileSync(inFile, JSON.stringify(project(plan, kept)));
      const resources = original.resources.filter(r => kept.has(r.address));
      const ids = new Set(resources.map(r => r.identity.id));
      const relationships = original.relationships.filter(r => ids.has(r.from.id) && ids.has(r.to.id));
      if (!isDeepStrictEqual(graph(inFile), {...original, resources, relationships})) return 0;
      return score(report(inFile));
    }
    let kept = new Set(deps.keys());
    let attempts = 0;
    for (let parts = 2; kept.size > 1 && attempts < budget;) {
      const order = [...kept];
      const chunk = Math.ceil(order.length / parts);
      let reduced = false;
      for (let offset = 0; offset < order.length && attempts < budget; offset += chunk) {
        const trial = new Set(kept);
        for (const address of order.slice(offset, offset + chunk)) trial.delete(address);
        let previous;
        do {
          previous = trial.size;
          for (const address of trial)
            if ([...deps.get(address)].some(target => !trial.has(target))) trial.delete(address);
        } while (trial.size < previous);
        attempts++;
        if (trial.size && measure(trial) >= threshold) {
          kept = trial;
          parts = Math.max(2, parts - 1);
          reduced = true;
          break;
        }
      }
      if (!reduced) {
        if (parts >= order.length) break;
        parts = Math.min(order.length, parts * 2);
      }
    }
    if (kept.size === deps.size) throw new Error('No verified reduction within the trial budget');
    const finalScore = measure(kept);
    if (finalScore < threshold) throw new Error('Final replay lost the selected finding or architecture evidence');
    fs.writeFileSync(output, JSON.stringify(project(plan, kept), null, 2) + '\n', {flag:'wx', mode:0o600});
    console.log(JSON.stringify({resourcesBefore:deps.size, resourcesAfter:kept.size, attempts,
      category:finding.category, findingIndex:index, pairValueBefore:threshold, pairValueAfter:finalScore}));
  } finally {
    fs.rmSync(scratch, {recursive:true, force:true});
  }
}

try {
  const [binary,input,index,output,budget = '64', ...extra] = process.argv.slice(2);
  if (!binary || !input || !/^\d+$/.test(index) || !output || extra.length || !/^\d+$/.test(budget) ||
      Number(budget) < 1 || Number(budget) > 128)
    throw new Error('Usage: node scripts/minimize-layout-fixture.cjs BINARY INPUT FINDING_INDEX NEW_OUTPUT [TRIALS 1..128]');
  minimize(path.resolve(binary), input, Number(index), output, Number(budget));
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
