function metadata(svg, byId) {
  const resources = JSON.parse(byId('viewer-data').textContent).filter(r => byId(r.id));
  const resourceIds = new Set(resources.map(r => r.id));
  const hex = text => Array.from(new TextEncoder().encode(text), byte => byte.toString(16).padStart(2, '0')).join('');
  const addressParts = address => {
    const parts = []; let start = 0, depth = 0, quoted = false, escaped = false;
    for (let i = 0; i < address.length; i++) {
      const c = address[i];
      if (escaped) escaped = false;
      else if (quoted && c === '\\') escaped = true;
      else if (depth && c === '"') quoted = !quoted;
      else if (!quoted && c === '[') depth++;
      else if (!quoted && c === ']') depth--;
      else if (!quoted && !depth && c === '.') { parts.push(address.slice(start, i)); start = i + 1; }
    }
    parts.push(address.slice(start));
    let boundary = 0;
    while (parts[boundary] === 'module' && boundary + 1 < parts.length) boundary += 2;
    return { module: boundary ? parts.slice(0, boundary).join('.') : 'root', type: parts[boundary + (parts[boundary] === 'data' ? 1 : 0)], mode: parts[boundary] === 'data' ? 'data' : 'managed' };
  };
  for (const metadata of svg.querySelectorAll('metadata[id$="architecture-relationships"]')) {
    const state = metadata.closest('[data-state-id]')?.dataset.stateId || '';
    const prefix = state ? `state-${hex(state)}-` : '';
    const scope = metadata.parentNode;
    const rows = Array.from(scope.querySelectorAll('[data-relationship-index] [data-source-address]'));
    for (const relation of JSON.parse(metadata.textContent)) {
      for (const source of relation.provenance.filter(p => p.kind === 'resource')) {
        const suffix = hex(source.address) + (source.deposed_key === null ? '-current' : '-deposed-' + hex(source.deposed_key));
        const id = `${prefix}resource-${suffix}`;
        if (resourceIds.has(id)) continue;
        const row = rows.find(row => row.dataset.sourceIdentity === 'terraform:' + suffix);
        if (!row) continue;
        row.id = id;
        resourceIds.add(id);
        const parts = addressParts(source.address);
        resources.push({ id, address: source.address, qualified: parts.module === 'root' ? 'module.root.' + source.address : source.address, identity: 'terraform:' + suffix, ...parts, state, deposed: source.deposed_key, relationship: true });
      }
    }
  }
  const targets = new Map(resources.map(r => [r.id, r]));
  const title = el => el.querySelector(':scope > title')?.textContent || '';
  const label = r => `${r.state ? r.state + ':' : ''}${r.address} · ${r.mode}${r.deposed ? ' · deposed: ' + r.deposed : ''}`;
  const edges = Array.from(svg.querySelectorAll('path[data-edge-kind][data-source][data-target]'));
  const links = Array.from(svg.querySelectorAll('a[href^="#"]')).map(el => ({ el, target: el.getAttribute('href').slice(1) })).filter(link => byId(link.target));
  const components = Array.from(svg.querySelectorAll('[data-entity-kind="synthetic"][data-component-id]'));
  const relationshipEntries = new Map(Array.from(svg.querySelectorAll('[data-relationship-entry][id]')).map(el => [el.id, el]));
  const relationshipEdges = id => edges.filter(edge => (edge.dataset.relationshipIds || '').split(' ').includes(id));

  return { resources, targets, title, label, edges, links, components, relationshipEntries, relationshipEdges };
}
