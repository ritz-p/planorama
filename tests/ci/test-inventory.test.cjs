const {test} = require('node:test');
const assert = require('node:assert/strict');
const {compare} = require('../../scripts/test-inventory.cjs');
const {measurements} = require('../../scripts/test-inventory/mutation-report.cjs');

test('inventory rejects missing, stale, duplicate and reclassified cases', () => {
  const a = {source:'suite.rs',id:'case',kind:'unit'};
  for (const [rows, cases, error] of [
    [[],[a],/Missing/], [[a],[],/Stale/], [[a,a],[a],/Duplicate/],
    [[a],[a,a],/Duplicate/], [[a],[{...a,kind:'property'}],/Changed kind/],
  ]) assert.throws(()=>compare(rows,cases,['kind']),error);
});

test('mutation results retain suite scope and distinguish check-only from a survivor', () => {
  const baseline = {scenario:'Baseline',summary:'Success'};
  const mutant = {file:'src/lib.rs',span:{start:{line:4,column:2}},function:{function_name:'predicate'},genre:'FnValue',replacement:'true'};
  const report = {total_mutants:2,outcomes:[baseline,...['Success','MissedMutant'].map(summary=>({scenario:{Mutant:mutant},summary}))]};
  const rows = measurements(report,'a'.repeat(40),'cargo test --lib','pilot artifact');
  assert.deepEqual(rows.map(r=>r.slice(2,4)),[['cargo test --lib','not-evaluated'],['cargo test --lib','survived']]);
  assert.throws(()=>measurements({...report,outcomes:report.outcomes.slice(1)},'a'.repeat(40),'suite','report'),/baseline/);
});

test('maintained inventories match local test registrations', () => {
  const {loadTables} = require('../../scripts/test-inventory.cjs');
  const {registrations} = require('../../scripts/test-inventory/registrations.cjs');
  compare(loadTables().rows,registrations(),['kind']);
});

test('mutation history distinguishes latest misses, improvements, repeats and unordered suite results', () => {
  const {mutationHistory} = require('../../scripts/test-inventory/history.cjs');
  const row = (mutation, revision, result, suite = 'default') => ({behavior:'behavior', mutation, revision, result, suite});
  const report = mutationHistory([
    row('improvement','b','killed','extended'), row('improvement','a','survived'),
    row('miss','b','survived'), row('miss','a','survived'),
    row('repeat','a','killed'), row('repeat','b','killed'),
    row('unknown','unrecorded','survived'), row('unknown','b','killed'),
    row('failed','b','timeout'), row('pending','a','not-evaluated'),
    row('conflict','b','killed','extended'), row('conflict','b','survived'),
  ], (a,b) => a === 'a' && b === 'b');
  assert.deepEqual(report.counts,{missed:1, improved:1, always:1, uncertain:4});
  assert.equal(report.measurements,12);
  assert.deepEqual(report.histories[0].records.map(r => [r.suite,r.line]), [['default',3],['extended',2]]);
  assert.equal(report.histories.find(g => g.mutation === 'unknown').ordered,false);
});
