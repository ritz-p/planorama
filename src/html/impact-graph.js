function impactGraph(index, svg) {
  const key = (state, identity) => JSON.stringify([state, identity]);
  const visible = index.resources.filter(r => !index.available || index.available(r.id));
  const nodes = new Map(visible.map(r => [key(r.state, r.identity), r]));
  const domKeys = new Map(visible.map(r => [r.id, key(r.state, r.identity)]));
  const addresses = new Map(index.resources.filter(r => !r.deposed).map(r => [key(r.state, r.address), key(r.state, r.identity)]));
  const edges = new Map();
  let scanned = 0, truncated = false;
  function add(edge) {
    if (++scanned > 10000) { truncated = true; return false; }
    edge.id = JSON.stringify([edge.from, edge.to, edge.kind, edge.direction, edge.semantics, edge.provenance]);
    edges.set(edge.id, edge);
    return true;
  }
  for (const metadata of svg.querySelectorAll('metadata[id$="architecture-relationships"]')) {
    const state = metadata.closest('[data-state-id]')?.dataset.stateId || '';
    for (const relation of JSON.parse(metadata.textContent)) {
      const proof = relation.provenance || [];
      const semantics = relation.inferred ? 'inferred' :
        relation.kind === 'dependency' && relation.directionality === 'directed' && proof.some(p => p.kind === 'reference') ? 'dependency' :
        relation.kind === 'association' && ['directed', 'undirected'].includes(relation.directionality) && proof.some(p => p.kind === 'resource') ? 'association' : 'unknown';
      if (!add({ from: key(state, relation.from), to: key(state, relation.to), kind: relation.kind,
        direction: relation.directionality, semantics, provenance: proof })) break;
    }
    if (truncated) break;
  }
  const cross = svg.querySelector('metadata[id="cross-state-relationships"]');
  if (cross && !truncated) {
    for (const relation of JSON.parse(cross.textContent)) {
      const resolve = (state, address) => addresses.get(key(state, address)) || key(state, 'unavailable:' + address);
      if (!add({ from: resolve(relation.from_state, relation.from), to: resolve(relation.to_state, relation.to),
        kind: 'dependency', direction: 'directed', semantics: 'dependency',
        provenance: [{ kind: 'remote-state', remote: relation.remote, output: relation.output }] })) break;
    }
  }
  const adjacent = new Map();
  for (const edge of [...edges.values()].sort((a, b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0)) {
    for (const endpoint of new Set([edge.from, edge.to])) {
      if (!adjacent.has(endpoint)) adjacent.set(endpoint, []);
      adjacent.get(endpoint).push(edge);
    }
  }
  return { nodes, domKeys, adjacent, truncated };
}

function exploreImpact(graph, origin, depth, direction, associations) {
  depth = Math.max(1, Math.min(3, Number(depth) || 1));
  const found = new Map([[origin, { key: origin, depth: 0, path: [] }]]);
  const traversed = new Map(), excluded = new Map();
  let scanned = 0, truncated = graph.truncated;
  const queue = [found.get(origin)];
  outer: for (let cursor = 0; cursor < queue.length; cursor++) {
    const current = queue[cursor];
    if (current.depth === depth || !graph.nodes.has(current.key)) continue;
    for (const edge of graph.adjacent.get(current.key) || []) {
      if (++scanned > 2000) { truncated = true; break outer; }
      const eligible = edge.semantics === 'dependency' || (associations && edge.semantics === 'association');
      if (!eligible) { if (excluded.size < 50) excluded.set(edge.id, edge); continue; }
      const outgoing = edge.from === current.key;
      if (edge.direction !== 'undirected' && ((direction === 'upstream' && outgoing) || (direction === 'downstream' && !outgoing))) continue;
      const next = outgoing ? edge.to : edge.from;
      if ((!found.has(next) && found.size === 200) || (!traversed.has(edge.id) && traversed.size === 400)) {
        truncated = true; break outer;
      }
      traversed.set(edge.id, edge);
      if (found.has(next)) continue;
      const entry = { key: next, depth: current.depth + 1, path: [...current.path, { edge, from: current.key, to: next }] };
      found.set(next, entry);
      queue.push(entry);
    }
  }
  return { found, traversed, excluded, scanned, truncated };
}
