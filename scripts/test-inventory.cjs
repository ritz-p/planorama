const fs = require('node:fs');
const path = require('node:path');
const {registrations, cargoRegistrations} = require('./test-inventory/registrations.cjs');

const root = path.resolve(__dirname, '..');
const directory = path.join(root, 'tests/inventory');
const columns = ['area','behavior','input','expected','source','case','runner'];
const mutationColumns = ['behavior','mutation','suite','result','revision','evidence'];
const kinds = ['unit','integration','property','benchmark','wasm','browser','ci','tooling','emulator','compatibility','layout'];
const mutationStates = ['killed','survived','timeout','unviable','not-evaluated','inconclusive'];
const key = record => record.source + '\t' + record.id;

function readTable(name, header) {
  const lines = fs.readFileSync(path.join(directory, name+'.tsv'), 'utf8').trimEnd().split(/\r?\n/);
  if (lines.shift() !== header.join('\t')) throw Error(name + ': unexpected TSV header');
  return lines.map((line, i) => {
    const fields = line.split('\t');
    if (fields.length !== header.length || fields.some(s => !s.trim())) throw Error(name+':'+(i+2)+': invalid fields');
    return Object.fromEntries(header.map((field, index) => [field, fields[index]]));
  });
}

function loadTables() {
  const tables = new Set([...kinds, 'mutation-results'].map(name => name + '.tsv'));
  for (const file of fs.readdirSync(directory)) {
    if (file.endsWith('.tsv') && !tables.has(file)) throw Error('Unknown inventory table: ' + file);
  }
  const rows = kinds.flatMap(kind => readTable(kind, columns).map(row => ({...row, kind, id:row.case})));
  for (const source of new Set(rows.map(row => row.source))) {
    if (!fs.existsSync(path.join(root, source))) throw Error('Missing source: ' + source);
  }
  const mutations = readTable('mutation-results',mutationColumns);
  const seen = new Set();
  for (const row of mutations) {
    const id = [row.revision,row.mutation,row.suite].join('\t');
    if (seen.has(id) || !mutationStates.includes(row.result) || !/^(?:[a-f0-9]{40}|unrecorded)$/.test(row.revision)) {
      throw Error('Invalid or duplicate mutation result: '+id);
    }
    seen.add(id);
  }
  return {rows, mutations};
}

function index(rows, label) {
  const result = new Map();
  for (const row of rows) {
    if (result.has(key(row))) throw Error('Duplicate '+label+': '+key(row));
    result.set(key(row),row);
  }
  return result;
}

function compare(rows, cases, fields) {
  const inventory = index(rows,'inventory row');
  const registered = index(cases,'registration');
  const errors = [];
  for (const [id, entry] of registered) {
    const row = inventory.get(id);
    if (!row) errors.push('Missing: '+id);
    else for (const field of fields) {
      if (row[field] !== entry[field]) errors.push('Changed '+field+': '+id+' → '+entry[field]);
    }
  }
  for (const id of inventory.keys()) if (!registered.has(id)) errors.push('Stale: '+id);
  if (errors.length) throw Error(errors.join('\n'));
}

function check(rows, cargo) {
  compare(rows, registrations(), ['kind']);
  if (cargo) {
    const cases = cargoRegistrations().map(c => ({...c, runner:c.runner+(c.ignored ? ' [ignored]' : '')}));
    compare(rows.filter(r => r.source.endsWith('.rs')), cases, ['runner']);
  }
}

function html({rows, mutations}) {
  const {mutationHistory} = require('./test-inventory/history.cjs');
  const history = mutationHistory(mutations);
  const revision = require('node:child_process').execFileSync('git', ['rev-parse','HEAD'], {encoding:'utf8'}).trim();
  const output = path.join(root,'output/test-inventory');
  fs.mkdirSync(output,{recursive:true});
  const template = fs.readFileSync(path.join(__dirname,'test-inventory/view.html'),'utf8');
  const data = JSON.stringify({rows, history, revision}).replaceAll('<','\\u003c');
  fs.writeFileSync(path.join(output,'index.html'),template.replace('INVENTORY_DATA', () => data));
  if (process.env.GITHUB_STEP_SUMMARY) {
    const {counts, histories, measurements} = history;
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## Mutation inventory\n\nGenerated from ${revision}. ${measurements} measurements of ${histories.length} distinct behavior/mutation pairs.\n\n| Latest missed | Detected after a miss | Always detected | Other or uncertain |\n| ---: | ---: | ---: | ---: |\n| ${counts.missed} | ${counts.improved} | ${counts.always} | ${counts.uncertain} |\n`);
  }
}

function main([command, flag, ...extra]) {
  if (extra.length || (flag && flag !== '--cargo') || !['check','html','list'].includes(command)) {
    throw Error('Usage: node scripts/test-inventory.cjs check [--cargo] | html | list [--cargo]');
  }
  process.chdir(root);
  if (command === 'list') {
    const cases = flag ? cargoRegistrations() : registrations();
    for (const r of cases) console.log([r.kind || 'cargo',r.source,r.id,r.runner || '',r.ignored ? 'ignored' : ''].join('\t'));
    return;
  }
  const tables = loadTables();
  check(tables.rows, flag === '--cargo');
  if (command === 'html') html(tables);
  console.log(tables.rows.length+' test cases, '+tables.mutations.length+' mutation results; '+command+' OK');
}

module.exports = {compare,loadTables};

if (require.main === module) {
  try { main(process.argv.slice(2)); } catch (e) { console.error(e.message); process.exitCode = 1; }
}
