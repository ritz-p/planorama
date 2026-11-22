function shortestRelationshipPath(graph, start, end) {
  const queue = [start], distance = new Map([[start, 0]]), ways = new Map([[start, 1]]);
  const previous = new Map();
  let scanned = 0, limited = graph.truncated;
  search: for (let cursor = 0; cursor < queue.length; cursor++) {
    const here = queue[cursor], depth = distance.get(here);
    if (distance.has(end) && depth >= distance.get(end)) break;
    for (const edge of graph.adjacent.get(here) || []) {
      if (++scanned > 4000) { limited = true; break search; }
      if (!['dependency', 'association', 'connection', 'subscription', 'eventsource', 'eventtarget'].includes(edge.kind)
          || !edge.provenance.length || !edge.provenance.every(p => ['reference', 'resource', 'remote-state'].includes(p.kind))) continue;
      if (edge.direction !== 'undirected' && !(edge.direction === 'directed' && edge.from === here)) continue;
      const next = edge.from === here ? edge.to : edge.from;
      if (!graph.nodes.has(next)) continue;
      if (distance.has(next)) {
        if (distance.get(next) === depth + 1) ways.set(next, Math.min(2, ways.get(next) + ways.get(here)));
        continue;
      }
      if (queue.length >= 500 || depth >= 12) { limited = true; continue; }
      distance.set(next, depth + 1); ways.set(next, ways.get(here));
      previous.set(next, {here, next, edge}); queue.push(next);
    }
  }
  const steps = [];
  if (distance.has(end)) {
    for (let current = end; current !== start;) {
      const step = previous.get(current);
      steps.unshift(step); current = step.here;
    }
  }
  return {steps, multiple: ways.get(end) > 1, limited};
}

function relationshipPath(context, index, coordinator) {
  const {byId, svg} = context;
  const status = byId('path-status'), origin = byId('path-origin'), steps = byId('path-steps');
  let start;
  function reset() {
    for (const el of svg.querySelectorAll('[data-path-step]')) el.removeAttribute('data-path-step');
    steps.replaceChildren(); status.textContent = '';
  }
  byId('path-start').addEventListener('click', () => {
    reset();
    const resource = index.targets.get(coordinator.selected);
    if (!resource) { status.textContent = 'Select a resource first.'; return; }
    start = resource.id;
    origin.textContent = 'From ' + index.label(resource);
  });
  byId('path-clear').addEventListener('click', () => {
    reset(); start = null; origin.textContent = '';
  });
  byId('path-find').addEventListener('click', () => {
    reset();
    const graph = impactGraph(index, svg);
    const from = graph.domKeys.get(start), to = graph.domKeys.get(coordinator.selected);
    if (!from || !to || from === to) {
      status.textContent = 'Set a visible starting resource, then select a different destination.'; return;
    }
    const result = shortestRelationshipPath(graph, from, to);
    const limit = result.limited ? ' Search limit reached.' : '';
    if (!result.steps.length) { status.textContent = 'No verified path found in the searched relationships.' + limit; return; }
    status.textContent = (result.multiple ? 'Multiple shortest paths found. Showing one.' : 'Showing one recorded relationship path.') + limit;
    const marked = new Set();
    for (const {here, next, edge} of result.steps) {
      const a = graph.nodes.get(here), b = graph.nodes.get(next);
      byId(a.id).setAttribute('data-path-step', 'true'); byId(b.id).setAttribute('data-path-step', 'true');
      const row = document.createElement('li');
      row.textContent = index.label(a) + (edge.direction === 'directed' ? ' → ' : ' ↔ ') + index.label(b) + ' · ' + edge.kind;
      const proofs = new Set(edge.provenance.filter(p => p.kind === 'resource').map(index.sourceIdentity));
      for (const el of index.edges.slice(0, 10000)) {
        if (el.dataset.edgeKind !== edge.kind || graph.domKeys.get(el.dataset.source) !== edge.from
          || graph.domKeys.get(el.dataset.target) !== edge.to) continue;
        const ids = (el.dataset.relationshipIds || '').split(' ').filter(Boolean);
        if (proofs.size && !ids.some(id => [...(index.relationshipEntries.get(id)?.querySelectorAll('[data-source-identity]') || [])]
          .some(r => proofs.has(r.dataset.sourceIdentity)))) continue;
        if (marked.has(el)) continue;
        marked.add(el); el.setAttribute('data-path-step', 'true');
        const numbers = ids.map(id => index.relationshipEntries.get(id)?.dataset.relationshipEntry).filter(Boolean);
        if (numbers.length) row.append(' · #' + numbers.join(', #'));
      }
      steps.append(row);
    }
  });
  return {reset};
}
