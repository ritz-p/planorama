const fs = require('node:fs');
const path = require('node:path');
const {createRequire} = require('node:module');
const {compileFunction} = require('node:vm');
const {execFileSync} = require('node:child_process');

function files(dir) {
  return fs.readdirSync(dir, {withFileTypes:true}).flatMap(entry => {
    const file = path.posix.join(dir, entry.name);
    return entry.isDirectory() ? files(file) : [file];
  }).sort();
}

function rustCases(source, text) {
  const cases = [];
  const pattern = /^([ \t]*)#\[test\][ \t]*\n((?:[ \t]*#\[[^\n]*\]\s*)*)\s*fn\s+(\w+)\s*\(/gm;
  for (const match of text.matchAll(pattern)) {
    const body = text.slice(match.index).split(new RegExp('^' + match[1] + '\\}', 'm'), 1)[0];
    const property = /\bproptest!\s*[(\{]/.test(body) || /\b\w+ in /.test(body.slice(body.indexOf('fn ')).split('{', 1)[0]);
    const kind = property ? 'property' : source.startsWith('tests/integration/') ? 'integration'
      : source.startsWith('tests/benchmarks') && match[2].includes('#[ignore') ? 'benchmark' : 'unit';
    cases.push({kind, source, id:match[3], body, ignored:match[2].includes('#[ignore')});
  }
  return cases;
}

function nodeCases(source, text, kind) {
  const cases = [];
  function register(id, ...args) {
    const fn = args.at(-1);
    if (typeof id !== 'string' || typeof fn !== 'function') throw Error('Unsupported registration: ' + source);
    cases.push({kind, source, id, body:fn.toString()});
  }
  register.test = register;
  const filename = path.resolve(source);
  const local = createRequire(filename);
  const load = Object.assign(name => name === 'node:test' ? register : local(name), local);
  const module = {exports:{}};
  compileFunction(text, ['require','module','exports','__dirname','__filename'], {filename})(
    load, module, module.exports, path.dirname(filename), filename);
  return cases;
}

function browserCases(source, text) {
  if (source.endsWith('/viewer.cjs')) {
    const {ViewerTests} = require(path.resolve(source));
    return Object.getOwnPropertyNames(ViewerTests.prototype).filter(id => id.startsWith('test_'))
      .map(id => ({source, id, kind:'browser', body:ViewerTests.prototype[id].toString()}));
  }
  if (!text.includes('assert') && !text.includes('assertEqual')) return [];
  const kind = path.basename(source).startsWith('wasm') || /(?:wasm|binding\.|Worker)/i.test(text) ? 'wasm' : 'browser';
  const cases = [];
  let rest = text;
  const strings = value => [...value.matchAll(/['"]([^'"]+)['"]/g)].map(m => m[1]);
  const scenarios = [...text.matchAll(/^([ \t]*)for\s*\(const scenario of (\[[^\]]+\]|require\('\.\.\/\.\.\/scripts\/cloudflare-capture\.cjs'\)\.cases)\)/gm)];
  const fixtures = source.endsWith('/wasm-service-audits-node.cjs')
    ? require('../../tests/support/wasm-service-audit-cases.cjs').map(c => c.fixture) : [''];
  const svgOnly = new Set(strings(text.match(/const svgOnly = new Set\(\[([^\]]*)/s)?.[1] || ''));
  for (const match of scenarios) {
    const block = text.slice(match.index).split(new RegExp('^'+match[1]+'\\}', 'm'),1)[0];
    rest = rest.replace(block,'');
    const names = match[2].startsWith('[') ? strings(match[2]) : require('../cloudflare-capture.cjs').cases;
    const base = block.match(/tests\/fixtures\/([\w/-]+)\.json/)?.[1];
    const enclosing = text.slice(0,match.index).match(/for \(const fixture of (\[[^\]]+\])\) \{\s*$/);
    for (const entry of enclosing ? strings(enclosing[1]) : fixtures) for (const name of names) {
      const skip = block.match(/if \(scenario === '([^']+)' && fixture !== '([^']+)'\) continue;/);
      if (skip && name === skip[1] && entry !== skip[2]) continue;
      const fixture = entry || base || '';
      const formats = block.match(/for\s*\(const format of (\[[^\]]*\]|formatsFor\(scenario\))/)?.[1];
      const variants = formats?.startsWith('[') ? strings(formats) : formats ? (svgOnly.has(name) ? ['svg'] : ['svg','html']) : [''];
      for (const format of variants) {
        cases.push({source, kind, id:[fixture,name,format].filter(Boolean).join('/'), body:block, scenario:name, fixture, format});
      }
    }
  }
  for (const m of [...rest.matchAll(/^([ \t]*)for \(const fixture of (\[[^\]]+\])\) \{/gm)].reverse()) {
    const block = rest.slice(m.index).split(new RegExp('^'+m[1]+'\\}', 'm'),1)[0];
    const formats = block.match(/for \(const format of (\[[^\]]*\])\)/);
    if (!formats) continue;
    rest = rest.replace(block, '');
    for (const fixture of strings(m[2])) for (const format of strings(formats[1])) {
      cases.push({source,kind,id:fixture+'/'+format,fixture,format,scenario:'fixture parity',body:block});
    }
  }
  if (!cases.length || /\bassert[.(]/.test(rest)) {
    cases.push({source,kind,id:path.basename(source,'.cjs'),body:rest});
  }
  return cases;
}

function matrixCases() {
  const cases = [];
  const add = (kind, source, id, behavior, input, expected) => cases.push({kind, source, id, behavior, input, expected});
  const quality = 'tests/layout-quality/manifest.json';
  for (const f of JSON.parse(fs.readFileSync(quality)).fixtures) {
    add('layout',quality,f.id,'Layout quality regression',f.args.join(' '),
      'Per-fixture gated metrics and stable edge-pair deltas remain within their budgets');
    add('layout','scripts/presentation-quality.cjs',f.id,'Rendered SVG geometry',f.id+'.svg from layout-quality',
      'Chromium finds text and frames, completes its comparison and reports no overlap or overflow');
  }
  const stability = 'tests/layout-stability/pairs.cjs';
  for (const pair of require(path.resolve(stability)).pairs) {
    add('layout',stability,pair.id,'Layout stability: '+pair.description, pair.description,
      pair.control ? 'Identical inputs produce no displacement' : 'Report retained frames and edge changes relative to the state scope (measurement)');
  }
  const terraform = 'tests/fixtures/compatibility/matrix.json';
  const matrix = JSON.parse(fs.readFileSync(terraform));
  for (const item of matrix.cases) for (const core of item.cores || Object.keys(matrix.terraform)) {
    for (const format of ['svg','html']) {
      add('compatibility','scripts/compatibility-output.cjs',[core,item.name,format].join('/'),
        'Cross-version diagram output', 'Render '+core+'/'+item.name+'/plan.json as '+format,
        'Rendered content matches the case baseline after its version-specific normalization');
    }
    add('compatibility',terraform,core+'/'+item.name,'Terraform plan compatibility',item.source+' with Terraform '+core+' / '+(item.provider || 'built-in provider'),
      'Captured plan, tool versions and source inputs satisfy the compatibility contract');
  }
  for (const [core, provider] of require('../../tests/support/provider-plan-pairs.cjs').pairs) {
    add('compatibility','tests/support/provider_plan.cjs',core+'/'+provider,'Saved-plan AWS account evidence',
      'Verify static, dynamic, variable and mismatched saved plans with Terraform '+core+' and AWS '+provider,
      'Only verified static bindings show an account; incompatible plans are rejected');
  }
  const cloudflare = 'tests/fixtures/cloudflare-matrix/matrix.json';
  const cf = JSON.parse(fs.readFileSync(cloudflare));
  for (const core of cf.terraform) for (const provider of Object.keys(cf.provider)) {
    add('compatibility',cloudflare,[core,provider,cf.case].join('/'),'Cloudflare provider compatibility',cf.source+' with Terraform '+core+' / provider '+provider,
      'Captured managed-create plan and schema match the pinned matrix');
  }
  for (const scenario of require('../cloudflare-capture.cjs').cases) {
    add('compatibility','scripts/cloudflare-capture.cjs',scenario,'Cloudflare capture reproducibility',
      'Check normalized '+scenario+'.json against capture metadata and the provider lock',
      'Plan is complete and normalized with the recorded Terraform version, format 1.2 and Cloudflare provider 5.28.1');
  }
  for (const mode of ['pr','full']) {
    const full = mode === 'full';
    add('emulator','scripts/aws-emulator/check-samples.sh',mode,'Reproduce AWS emulator samples',
      full ? 'Capture queue, containment and large samples' : 'Capture containment with --create-only',
      'Containment create snapshot and SVG match the checked-in examples'+(full ? ' and the large plan matches the stress fixture' : ''));
  }
  return cases;
}

function registrations() {
  const cases = matrixCases();
  for (const source of [...files('src'), ...files('tests'), ...files('examples').filter(p => p.endsWith('.rs'))]) {
    if (source.endsWith('.rs')) cases.push(...rustCases(source, fs.readFileSync(source,'utf8')));
    else if (source.endsWith('.test.cjs')) {
      const dir = source.split('/')[1];
      cases.push(...nodeCases(source, fs.readFileSync(source,'utf8'), dir === 'support' ? 'tooling' : dir));
    } else if (source.startsWith('tests/browser/') && source.endsWith('.cjs')) {
      cases.push(...browserCases(source, fs.readFileSync(source,'utf8')));
    } else if (/^tests\/emulator\/.*\.(cjs|sh)$/.test(source) || /^tests\/support\/verify-.*\.cjs$/.test(source)) {
      cases.push({source,kind:source.includes('/emulator/') ? 'emulator' : 'tooling',id:path.basename(source),body:fs.readFileSync(source,'utf8')});
    }
  }
  return cases;
}

function cargoRegistrations() {
  const run = (cmd,args,options = {}) => execFileSync(cmd,args,{encoding:'utf8',maxBuffer:64*1024*1024, ...options});
  const jsonLines = text => text.trim().split('\n').filter(Boolean).map(line => JSON.parse(line));
  const built = run('cargo',['test','--locked','--all-features','--all-targets','--no-run','--message-format=json']);
  const cases = [];
  for (const item of jsonLines(built).filter(i => i.executable && i.profile?.test)) {
    const listing = run(item.executable,['--list','--format','json','-Z','unstable-options'],
      {env:{...process.env,RUSTC_BOOTSTRAP:'1'}});
    for (const t of jsonLines(listing).filter(t => t.event === 'discovered')) {
      cases.push({source:path.relative(process.cwd(),path.resolve(t.source_path)), id:t.name.split('::').at(-1),
        runner:item.target.name+'::'+t.name, ignored:t.ignore});
    }
  }
  return cases;
}

module.exports = {registrations,cargoRegistrations};
