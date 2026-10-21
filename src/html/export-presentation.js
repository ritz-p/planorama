function exportPresentation(context, index, requested, provenanceSeeds) {
  const { svg, byId } = context;
  if (svg.querySelectorAll('*').length > 100000) throw new Error('Diagram exceeds the export presentation limit');
  const ids = new Set(requested), entries = new Set(), paths = new Set();
  const sources = new Map(index.resources.map(r => [JSON.stringify([r.state, r.identity]), r.id]));
  const entrySources = entry => [...entry.querySelectorAll('[data-source-identity]')].map(row =>
    sources.get(JSON.stringify([entry.closest('[data-state-id]')?.dataset.stateId || '', row.dataset.sourceIdentity]))).filter(Boolean);
  const add = id => {
    if (!index.targets.has(id) || !byId(id)) throw new Error('Required context is unavailable in this generated view');
    ids.add(id);
    if (ids.size > 200) throw new Error('Required context exceeds the 200-resource export limit');
  };
  let changed = true;
  while (changed) {
    const previous = ids.size + entries.size + paths.size;
    for (const id of ids) {
      for (const parent of index.targets.get(id).context || []) add(parent);
      for (let parent = byId(id).parentElement; parent && parent !== svg; parent = parent.parentElement) {
        if (index.targets.has(parent.id)) add(parent.id);
      }
    }
    for (const path of index.edges) {
      if (!ids.has(path.dataset.source) || !ids.has(path.dataset.target)) continue;
      paths.add(path);
      for (const entry of (path.dataset.relationshipIds || '').split(' ').filter(Boolean)) entries.add(entry);
    }
    for (const [id, entry] of index.relationshipEntries) {
      if (entrySources(entry).some(source => ids.has(source))) entries.add(id);
      if (!entries.has(id)) continue;
      for (const source of entrySources(entry)) add(source);
      for (const link of entry.querySelectorAll('[data-relationship-endpoint][href]')) add(link.getAttribute('href').slice(1));
    }
    if (paths.size > 400 || entries.size > 400) throw new Error('Required relationships exceed the 400-relationship export limit');
    changed = previous !== ids.size + entries.size + paths.size;
  }
  for (const id of entries) if (!index.relationshipEntries.has(id)) throw new Error('Relationship presentation is unavailable');
  const keep = new Set([...ids].map(byId));
  for (const path of paths) keep.add(path);
  for (const id of entries) keep.add(index.relationshipEntries.get(id));
  const pairs = new Set([...paths].map(path => JSON.stringify([path.dataset.source, path.dataset.target])));
  const state = el => el.closest('[data-state-id]')?.dataset.stateId || '';
  for (const el of svg.querySelectorAll('[data-container], [data-route-table], [data-member-cluster]')) {
    if (ids.has(el.dataset.container || el.dataset.routeTable || el.dataset.memberCluster)) keep.add(el);
  }
  for (const el of svg.querySelectorAll('[data-endpoints]')) {
    if (pairs.has(JSON.stringify([el.dataset.source, el.dataset.target]))) keep.add(el);
  }
  for (const el of svg.querySelectorAll('[data-junction-edges]')) {
    const prefix = el.closest('[data-state-id]')?.id.replace(/state$/, '') || '';
    if (JSON.parse(el.dataset.junctionEdges).some(([a, b]) => pairs.has(JSON.stringify([prefix + a, prefix + b])))) keep.add(el);
  }
  for (const el of svg.querySelectorAll('[data-relationship-marker]')) {
    if (entries.has(el.querySelector('a')?.getAttribute('href')?.slice(1))) keep.add(el);
  }
  for (const el of svg.querySelectorAll('[data-deployment-scope]')) {
    if ([...ids].some(id => {
      const r = index.targets.get(id), scope = r.scope;
      return r.state === state(el) && scope && scope.provider === el.dataset.scopeProvider &&
        (scope.account || '') === (el.dataset.account || '') &&
        (el.hasAttribute('data-account-panel') || (scope.region || '') === (el.dataset.region || '') && scope.global === el.hasAttribute('data-global'));
    })) keep.add(el);
  }
  for (const el of index.components) {
    for (const row of el.querySelectorAll('[data-component-member]')) {
      if ([...row.querySelectorAll('a[href]')].some(link => ids.has(link.getAttribute('href').slice(1)))) keep.add(row);
    }
  }
  const ancestors = new Set();
  for (const el of keep) for (let parent = el.parentElement; parent && parent !== svg; parent = parent.parentElement) ancestors.add(parent);
  const frames = '[data-annotation-area], [data-annotation-section], [data-annotation-card], [data-logical-group], [data-action-section], [data-entity-kind="synthetic"], [data-state-id]';
  const retained = new Map();
  function project(el, inside = false) {
    if (el.matches('metadata, script, foreignObject, [data-hotspot-overlay]')) return null;
    if (el.matches('[data-occluder]') && !ids.has(el.dataset.occluder)) return null;
    if (index.targets.has(el.id) && !ids.has(el.id)) return null;
    if (el.hasAttribute('data-relationship-entry') && !entries.has(el.id)) return null;
    if (el.hasAttribute('data-component-member') && !keep.has(el)) return null;
    if (el.matches('[data-edge-kind], [data-endpoints], [data-junction-edges], [data-relationship-marker]') && !keep.has(el)) return null;
    const atomic = inside || keep.has(el) || el.localName === 'defs';
    if (!atomic && el !== svg && !ancestors.has(el)) return null;
    const clone = el.cloneNode(false);
    clone.removeAttribute('data-junction-edges');
    retained.set(el, clone);
    for (const child of el.childNodes) {
      if (child.nodeType !== 1) {
        if (atomic) clone.append(child.cloneNode(true));
        continue;
      }
      const decoration = !atomic && el.matches(frames) && child.matches('rect, text, title') && !index.targets.has(child.id);
      const copied = project(child, atomic || decoration);
      if (copied) clone.append(copied);
    }
    return clone;
  }
  const output = project(svg);
  const outputIds = new Set([...output.querySelectorAll('[id]')].map(el => el.id));
  for (const link of output.querySelectorAll('a[href]')) {
    const href = link.getAttribute('href');
    if (href.startsWith('#') && !outputIds.has(href.slice(1))) link.removeAttribute('href');
  }
  for (const defs of svg.querySelectorAll('defs')) {
    if (retained.has(defs)) continue;
    const parent = retained.get(defs.parentElement);
    if (parent) parent.prepend(project(defs, true));
  }
  for (const [original, clone] of retained) {
    if (original.hasAttribute('data-action-section')) {
      const count = clone.querySelectorAll('[data-placement="logical"]').length;
      for (const heading of clone.querySelectorAll(':scope > text, :scope > title')) heading.textContent = heading.textContent.replace(/\(\d+\)/, `(${count})`);
    }
    if (original.hasAttribute('data-component-scope')) clone.textContent = 'Context for retained members';
  }
  for (const id of ids) {
    const matches = [...output.querySelectorAll('[id]')].filter(el => el.id === id);
    if (matches.length !== 1) throw new Error('Resource identity cannot be faithfully extracted');
  }
  const points = [];
  const rootMatrix = svg.getScreenCTM();
  if (!rootMatrix) throw new Error('Diagram geometry is unavailable');
  for (const [original] of retained) {
    if (!original.getBBox || original.closest('defs') || original === svg || original.localName === 'svg' || original.children.length && !['text', 'path', 'use'].includes(original.localName)) continue;
    const bounds = original.getBBox(), screen = original.getScreenCTM();
    if (!screen) throw new Error('Retained geometry is unavailable');
    const matrix = rootMatrix.inverse().multiply(screen);
    for (const [x, y] of [[bounds.x, bounds.y], [bounds.x + bounds.width, bounds.y], [bounds.x, bounds.y + bounds.height], [bounds.x + bounds.width, bounds.y + bounds.height]]) {
      points.push(new DOMPoint(x, y).matrixTransform(matrix));
    }
  }
  if (!points.length || !points.every(p => Number.isFinite(p.x) && Number.isFinite(p.y))) throw new Error('Invalid retained bounds');
  const bounds = points.reduce((b, p) => [Math.min(b[0], p.x), Math.min(b[1], p.y), Math.max(b[2], p.x), Math.max(b[3], p.y)], [Infinity, Infinity, -Infinity, -Infinity]);
  const left = bounds[0] - 24, top = bounds[1] - 24;
  const width = Math.max(800, bounds[2] - left + 24);
  const height = bounds[3] - top + 24;
  output.setAttribute('viewBox', [left, top - 80, width, height + 80].join(' '));
  const background = document.createElementNS(svg.namespaceURI, 'rect');
  for (const [name, value] of Object.entries({ x: left, y: top - 80, width, height: height + 80, fill: '#ffffff' })) background.setAttribute(name, value);
  output.prepend(background);
  const count = ids.size, contextCount = ids.size - requested.size + provenanceSeeds, pathCount = paths.size;
  return { svg: output, ids: [...ids], count, contextCount, pathCount, annotate(summary, relationships) {
    const metadata = document.createElementNS(svg.namespaceURI, 'metadata');
    metadata.textContent = JSON.stringify({ scope: 'resource-neighborhood', resources: [...ids], relationships, renderedPaths: pathCount, contextResources: contextCount, description: summary });
    output.prepend(metadata);
    const heading = document.createElementNS(svg.namespaceURI, 'text');
    heading.setAttribute('x', left + 24); heading.setAttribute('y', top - 52);
    heading.setAttribute('font-family', 'ui-monospace, Consolas, monospace'); heading.setAttribute('font-size', '13'); heading.setAttribute('fill', '#0f172a');
    const lines = [`Neighborhood extract: ${count} resources; ${pathCount} paths; ${contextCount} context/provenance.`,
      'Original layout, no repacking. Relationships are not runtime impact.',
      summary.includes('Traversal truncated') ? 'Traversal truncated; see document description for limits and scope.' : 'See document description for depth, relationship policy and unavailable neighbors.'];
    for (const [i, line] of lines.entries()) {
      const span = document.createElementNS(svg.namespaceURI, 'tspan'); span.setAttribute('x', left + 24); span.setAttribute('dy', i ? 20 : 0); span.textContent = line; heading.append(span);
    }
    output.append(heading);
  } };
}
