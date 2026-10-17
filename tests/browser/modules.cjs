const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const test = require('node:test');
const context = vm.createContext({});
for (const name of ['lifecycle', 'highlighting', 'impact-graph', 'neighborhood']) {
  vm.runInContext(fs.readFileSync(`src/html/${name}.js`, 'utf8'), context);
}

test('lifecycle resets each registered feature on every transition', () => {
  const lifecycle = context.lifecycle();
  const calls = [];
  lifecycle.register({ reset: () => calls.push('diagnostics') });
  lifecycle.register({ reset: () => calls.push('selection') });
  lifecycle.reset();
  lifecycle.reset();
  assert.deepEqual(calls, ['diagnostics', 'selection', 'diagnostics', 'selection']);
});

function graph(edges, ids = ['a', 'b', 'c']) {
  const adjacent = new Map(ids.map(id => [id, []]));
  for (const edge of edges) for (const id of new Set([edge.from, edge.to])) {
    if (!adjacent.has(id)) adjacent.set(id, []);
    adjacent.get(id).push(edge);
  }
  return { nodes: new Map(ids.map(id => [id, {}])), adjacent, truncated: false };
}
const edge = (from, to, id = from + to, semantics = 'dependency', direction = 'directed') => ({ from, to, id, kind: semantics, semantics, direction });
const reached = result => [...result.found.keys()];

test('neighborhood follows recorded non-containment relations without impact semantics', () => {
  const g = graph([edge('a', 'b'), edge('c', 'b', 'inferred', 'inferred'), { ...edge('a', 'd'), kind: 'containment' }, edge('b', 'missing')], ['a', 'b', 'c', 'd']);
  assert.deepEqual([...context.resourceNeighborhood(g, ['a'], 0).found], ['a']);
  assert.deepEqual([...context.resourceNeighborhood(g, ['a'], 1).found], ['a', 'b']);
  const result = context.resourceNeighborhood(g, ['a'], 2);
  assert.deepEqual([...result.found], ['a', 'b', 'c']);
  assert.deepEqual([...result.missing], ['missing']);
  assert.equal(result.relations.get('inferred').semantics, 'inferred');
  const ids = ['a', ...Array.from({ length: 250 }, (_, i) => 'n' + i)];
  const bounded = context.resourceNeighborhood(graph(ids.slice(1).map(id => edge('a', id)), ids), ['a'], 3);
  assert.equal(bounded.found.size, 200);
  assert.equal(bounded.truncated, true);
});

test('dependency arrows run upstream to downstream, depth is bounded', () => {
  const g = graph([edge('a', 'b'), edge('b', 'c')]);
  assert.deepEqual(reached(context.exploreImpact(g, 'a', 1, 'downstream', false)), ['a', 'b']);
  const result = context.exploreImpact(g, 'a', 2, 'downstream', false);
  assert.deepEqual(reached(result), ['a', 'b', 'c']);
  assert.equal(result.found.get('c').path.length, 2);
  assert.deepEqual(reached(context.exploreImpact(g, 'c', 2, 'upstream', false)), ['c', 'b', 'a']);
  assert.deepEqual(reached(context.exploreImpact(g, 'c', 3, 'downstream', false)), ['c']);
  assert.deepEqual(reached(context.exploreImpact(g, 'b', 3, 'both', false)), ['b', 'a', 'c']);
});

test('associations are opt-in and inferred/unknown relations never propagate', () => {
  const g = graph([edge('a', 'b', 'association', 'association', 'undirected'), edge('b', 'c', 'inferred', 'inferred'), edge('a', 'c', 'unknown', 'unknown')]);
  assert.deepEqual(reached(context.exploreImpact(g, 'a', 3, 'upstream', false)), ['a']);
  const result = context.exploreImpact(g, 'a', 3, 'upstream', true);
  assert.deepEqual(reached(result), ['a', 'b']);
  assert.equal(result.traversed.get('association').direction, 'undirected');
  assert.equal(result.excluded.get('inferred').semantics, 'inferred');
  assert.equal(result.excluded.get('unknown').semantics, 'unknown');
});

test('cycles, parallel provenance, missing nodes and fan-out are bounded', () => {
  const g = graph([edge('a', 'b', 'first'), edge('a', 'b', 'second'), edge('b', 'a'), edge('b', 'missing'), edge('missing', 'c')]);
  const result = context.exploreImpact(g, 'a', 3, 'downstream', false);
  assert.deepEqual(reached(result), ['a', 'b', 'missing']);
  assert.equal(result.traversed.size, 4);
  assert.equal(result.found.get('missing').depth, 2);
  const ids = ['a', ...Array.from({ length: 500 }, (_, i) => 'n' + i)];
  const bounded = context.exploreImpact(graph(ids.slice(1).map(id => edge('a', id)), ids), 'a', 3, 'both', true);
  assert.equal(bounded.found.size, 200);
  assert.equal(bounded.truncated, true);
  const parallel = context.exploreImpact(graph(Array.from({ length: 450 }, (_, i) => edge('a', 'b', String(i)))), 'a', 3, 'both', false);
  assert.equal(parallel.traversed.size, 400);
  assert.equal(parallel.truncated, true);
  const chain = ['a', 'b', 'c', 'd', 'e'];
  const clamped = context.exploreImpact(graph(chain.slice(1).map((id, i) => edge(chain[i], id)), chain), 'a', 99, 'downstream', false);
  assert.deepEqual(reached(clamped), ['a', 'b', 'c', 'd']);
});

test('metadata graph preserves state and parallel relationship identities', () => {
  const relation = { from: 'id:a', to: 'id:b', kind: 'dependency', directionality: 'directed', inferred: false, provenance: [{ kind: 'reference' }] };
  const metadata = ['one', 'two'].map(state => ({ textContent: JSON.stringify([relation, relation]), closest: () => ({ dataset: { stateId: state } }) }));
  const resources = ['one', 'two'].flatMap(state => ['a', 'b'].map(address => ({ state, identity: 'id:' + address, address, id: state + address })));
  const g = context.impactGraph({ resources }, { querySelectorAll: () => metadata, querySelector: () => null });
  const origin = g.domKeys.get('onea');
  const result = context.exploreImpact(g, origin, 3, 'both', false);
  assert.equal(result.found.size, 2);
  assert.equal(result.traversed.size, 1);
  assert.ok([...result.found.keys()].every(key => JSON.parse(key)[0] === 'one'));
});

test('highlight reset removes all selection roles without touching edge styling', () => {
  const attrs = new Map([['stroke', 'red'], ['data-viewer-edge', 'provenance'], ['data-viewer-related', 'true']]);
  const element = { removeAttribute: key => attrs.delete(key) };
  const viewport = new Map([['data-selection', 'true']]);
  context.highlighting({ querySelectorAll: () => [element] }, { removeAttribute: key => viewport.delete(key) }).clear();
  assert.deepEqual([...attrs], [['stroke', 'red']]);
  assert.equal(viewport.size, 0);
});
