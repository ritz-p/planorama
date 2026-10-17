function resourceNeighborhood(graph, origins, depth) {
  const found = new Set(origins), queue = [...origins].map(key => [key, 0]);
  const relations = new Map(), missing = new Set();
  let scanned = 0, truncated = graph.truncated;
  outer: for (let i = 0; i < queue.length; i++) {
    const [key, distance] = queue[i];
    if (distance >= depth) continue;
    for (const edge of graph.adjacent.get(key) || []) {
      if (++scanned > 2000) { truncated = true; break outer; }
      if (edge.kind === 'containment') continue;
      const next = edge.from === key ? edge.to : edge.from;
      if (!graph.nodes.has(next)) { missing.add(next); continue; }
      if ((!found.has(next) && found.size >= 200) || (!relations.has(edge.id) && relations.size >= 400)) { truncated = true; break outer; }
      relations.set(edge.id, edge);
      if (!found.has(next)) { found.add(next); queue.push([next, distance + 1]); }
    }
  }
  return { found, relations, missing, truncated };
}

function neighborhoodExport(context, index, coordinator, exporter) {
  const { svg, byId } = context;
  byId('export-neighborhood').addEventListener('click', () => {
    try {
      const selected = index.targets.get(coordinator.selected);
      if (!selected) throw new Error('Select a resource before exporting its neighborhood');
      const graph = impactGraph(index, svg);
      const origin = graph.domKeys.get(selected.id);
      const origins = new Set([origin]);
      if (selected.relationship) {
        for (const edges of graph.adjacent.values()) for (const edge of edges) {
          if (JSON.parse(edge.from)[0] === selected.state && edge.provenance.some(p => p.kind === 'resource' && index.sourceIdentity(p) === selected.identity)) {
            for (const key of [edge.from, edge.to]) if (graph.nodes.has(key)) origins.add(key);
          }
        }
      }
      if (origins.size > 200) throw new Error('Required provenance exceeds the 200-resource export limit');
      const depth = Math.max(0, Math.min(3, Number(byId('export-depth').value) || 0));
      const result = resourceNeighborhood(graph, origins, depth);
      const ids = new Set([...result.found].map(key => graph.nodes.get(key)?.id).filter(Boolean));
      const presentation = exportPresentation(context, index, ids, origins.size - 1);
      const summary = `${presentation.count} resources (${presentation.contextCount} added context/provenance), ${presentation.pathCount} rendered paths. Depth ${depth}, both directions; dependencies, associations and inferred connections retain their recorded meaning. ${result.missing.size} unavailable neighbors. ${result.truncated ? 'Traversal truncated at safety limits. ' : ''}Extract without repacking; original geometry retained with 24px padding.`;
      const retained = new Set(presentation.ids);
      const relationships = new Map();
      for (const edges of graph.adjacent.values()) for (const edge of edges) {
        if (retained.has(graph.nodes.get(edge.from)?.id) && retained.has(graph.nodes.get(edge.to)?.id)) relationships.set(edge.id, edge);
      }
      presentation.annotate(summary, [...relationships.values()]);
      exporter.download(presentation.svg, summary, 'planorama-neighborhood.svg');
    } catch (error) {
      byId('export-status').textContent = 'Export cancelled: ' + error.message;
    }
  });
}
