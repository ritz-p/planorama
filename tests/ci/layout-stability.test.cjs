const {test} = require('node:test');
const assert = require('node:assert/strict');
const {compare,patch} = require('../../scripts/layout-stability.cjs');
const id = (address, state = null, mode = 'managed', deposed = null) => ({state,address,mode,deposed});
const record = (identity,x=10,y=20,width=100,height=80,origin=[0,0]) => ({identity,bounds:{x,y,width,height},representation:'spatial',scopes:[{identity:{kind:'state',state:identity.state},origin}]});
const report = records => ({schema_version:1,category:'layout',resource_geometry:{schema_version:1,records}});

test('identical control, stable ordering and structured identities', () => {
  const records = [record(id('same','a')),record(id('same','b')),record(id('same','a','data')),record(id('same','a','managed','old'))];
  const result = compare(report(records),report([...records].reverse()));
  assert.equal(result.summary.matched,4);
  assert.equal(result.summary.displacement.whole_diagram.maximum,0);
  assert.equal(result.summary.width_change.maximum,0);
  assert.equal(result.summary.displacement.relative_to_shared_scope.maximum,0);
  assert.deepEqual(result,compare(report([...records].reverse()),report(records)));
});

test('global and named-state translation are separated from local churn and size', () => {
  const first = record(id('app','b'),10,20);
  const next = record(id('app','b'),10,120,130,90,[0,100]);
  const result = compare(report([first]),report([next]));
  assert.equal(result.matched[0].whole_diagram.dy,100);
  assert.equal(result.matched[0].relative_to_shared_scope.distance,0);
  assert.deepEqual(result.summary.uniform_translation_of_measured_resources,{dx:0,dy:100});
  assert.equal(result.matched[0].size.width,30);
  assert.equal(result.summary.unrelated_displacement.whole_diagram.maximum,100);
});

test('nearest shared containment scope is used and unrelated movement is explicit', () => {
  const first = record(id('app'),20,30), next = record(id('app'),120,140);
  const scope = {kind:'resource',resource:id('subnet')};
  first.scopes.unshift({identity:scope,origin:[10,20]});
  next.scopes.unshift({identity:scope,origin:[110,120]});
  const result = compare(report([first]),report([next]),{affected:[id('app')]});
  assert.equal(result.matched[0].relative_to_shared_scope.dy,10);
  assert.deepEqual(result.matched[0].relative_to_shared_scope.scope,scope);
  assert.equal(result.summary.unrelated_displacement.whole_diagram.count,0);
  next.scopes[0].identity = {kind:'resource',resource:id('different')};
  assert.equal(compare(report([first]),report([next])).matched[0].relative_to_shared_scope.scope.kind,'state');
});

test('added, removed, unmeasured and ambiguous identities never acquire invented coordinates', () => {
  const first = [record(id('removed')),record(id('logical')),record(id('duplicate')),record(id('duplicate'))];
  const next = [record(id('added')), {...record(id('logical')),bounds:null,representation:'logical',unavailable_reason:'logical'}, record(id('duplicate'))];
  const result = compare(report(first),report(next));
  assert.deepEqual(result.added,[id('added')]);
  assert.deepEqual(result.removed,[id('removed')]);
  assert.equal(result.unmeasured.length,1);
  assert.equal(result.unmatchable.length,1);
  assert.equal(result.matched.length,0);
  assert.equal(result.summary.displacement.whole_diagram.maximum,null);
  assert.throws(() => compare({},report([])),/contract/);
});

test('paired fixture patches are deterministic and never mutate the source', () => {
  const original = {values:[1,2],nested:{x:1}};
  const changes = [{op:'add',path:['values','-'],value:3},{op:'remove',path:['values',0]},{op:'replace',path:['nested','x'],value:2}];
  assert.deepEqual(patch(original,changes),{values:[2,3],nested:{x:2}});
  assert.deepEqual(original,{values:[1,2],nested:{x:1}});
  assert.throws(() => patch(original,[{op:'add',path:['__proto__'],value:{}}]),/Invalid/);
});

test('global setting changes do not claim that all resources are unrelated', () => {
  const result = compare(report([record(id('app'))]),report([record(id('app'),10,120)]),{intentional_restructure:true,affected:[]});
  assert.equal(result.matched[0].declared_unrelated,null);
  assert.equal(result.summary.unrelated_displacement.whole_diagram.count,0);
  assert.throws(() => compare(report([]),report([]),{affected:[{address:'incomplete'}]}),/affected/);
});
