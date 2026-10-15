function impact(context, index, selection, coordinator) {
  const { svg, byId } = context;
  const graph = impactGraph(index, svg);
  const status = byId('impact-status'), results = byId('impact-results');
  const depth = byId('impact-depth'), direction = byId('impact-direction'), associations = byId('impact-associations');
  const initial = status.textContent;
  function reset() {
    for (const el of svg.querySelectorAll('[data-impact-hop], [data-impact-edge]')) {
      el.removeAttribute('data-impact-hop'); el.removeAttribute('data-impact-edge');
    }
    results.replaceChildren(); status.textContent = initial;
  }
  const name = key => {
    const r = graph.nodes.get(key);
    return r ? `${index.label(r)} · ${r.action || 'action unknown'}` : `${key} · unavailable in this view`;
  };
  const edgeText = edge => `${name(edge.from)} ${edge.direction === 'undirected' ? '↔' : '→'} ${name(edge.to)} · ${edge.kind} / ${edge.semantics} / ${edge.direction || 'direction unknown'}`;
  function explore() {
    reset();
    const origin = graph.domKeys.get(coordinator.selected);
    if (!origin) { status.textContent = 'Select a resource to explore. Relationship index entries and component summaries are not traversal origins.'; return; }
    const result = exploreImpact(graph, origin, depth.value, direction.value, associations.checked);
    const selected = graph.nodes.get(origin);
    if (selected.relationship && !graph.adjacent.has(origin)) {
      status.textContent = 'This resource supplies relationship provenance. Select an endpoint in the selection details to explore its graph relationships; provenance is not a dependency edge.';
      return;
    }
    status.textContent = `Potentially related resources from ${name(origin)}: ${result.found.size - 1} within ${depth.value} hops, ${direction.options[direction.selectedIndex].text}. ${selected.action === 'unchanged' ? 'Origin is unchanged context. ' : ''}${result.truncated ? 'Exploration truncated by graph, node, edge or scan limits. ' : ''}Only this effective view is searched; filtered/unavailable resources are not evidence of absence. Dependencies point to dependents. Associations follow their recorded direction (undirected in either direction) and do not establish causality. No apply or runtime effects are predicted.`;
    const list = document.createElement('ol');
    for (const entry of result.found.values()) {
      if (!entry.depth) continue;
      const row = document.createElement('li');
      const resource = graph.nodes.get(entry.key);
      const label = `${entry.depth === 1 ? 'Direct' : 'Additional'} · hop ${entry.depth} · ${name(entry.key)}`;
      if (resource && byId(resource.id)) {
        const el = byId(resource.id); el.setAttribute('data-impact-hop', entry.depth);
        row.append(selection.button(label, () => selection.select(resource.id)));
      } else row.textContent = label;
      const path = document.createElement('details');
      const summary = document.createElement('summary'); summary.textContent = 'Discovery path from origin';
      path.append(summary);
      for (const step of entry.path) {
        const text = document.createElement('p');
        text.textContent = `Traversal ${name(step.from)} → ${name(step.to)}; recorded ${edgeText(step.edge)}; provenance ${JSON.stringify(step.edge.provenance)}`;
        path.append(text);
      }
      row.append(path); list.append(row);
    }
    results.append(list);
    const visitedPairs = new Map();
    for (const edge of result.traversed.values()) {
      const pair = JSON.stringify([edge.from, edge.to, edge.kind, edge.direction]);
      if (!visitedPairs.has(pair)) visitedPairs.set(pair, []);
      visitedPairs.get(pair).push(edge);
    }
    for (const edge of index.edges) {
      const from = graph.domKeys.get(edge.dataset.source), to = graph.domKeys.get(edge.dataset.target);
      const direction = edge.dataset.crossState === 'true' ? 'directed' : edge.dataset.directionality;
      const matches = visitedPairs.get(JSON.stringify([from, to, edge.dataset.edgeKind, direction])) || [];
      const sources = new Set((edge.dataset.relationshipIds || '').split(' ').flatMap(id =>
        Array.from(index.relationshipEntries.get(id)?.querySelectorAll('[data-source-identity]') || []).map(row => row.dataset.sourceIdentity)));
      if (matches.some(match => match.semantics === 'dependency' || match.provenance.some(p => p.kind === 'resource' && sources.has(index.sourceIdentity(p))))) edge.setAttribute('data-impact-edge', 'true');
    }
    const details = document.createElement('details');
    const heading = document.createElement('summary');
    heading.textContent = `${result.traversed.size} traversed relationships (including parallel edges and cycles)`;
    details.append(heading);
    for (const edge of result.traversed.values()) {
      const p = document.createElement('p'); p.textContent = `${edgeText(edge)}; provenance ${JSON.stringify(edge.provenance)}`; details.append(p);
    }
    results.append(details);
    if (result.excluded.size) {
      const details = document.createElement('details');
      const summary = document.createElement('summary'); summary.textContent = 'Excluded associations / inferred / unknown semantics (up to 50 examples)';
      details.append(summary);
      for (const edge of result.excluded.values()) {
        const p = document.createElement('p'); p.textContent = edgeText(edge); details.append(p);
      }
      results.append(details);
    }
  }
  byId('impact-explore').addEventListener('click', explore);
  for (const control of [depth, direction, associations]) control.addEventListener('change', reset);
  return { reset };
}
