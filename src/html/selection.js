function selection(context, index, navigation, highlights, coordinator) {
  const { svg, viewport, byId, details, results } = context;
  const { targets, label, title, relationshipEntries, relationshipEdges, links, edges, components } = index;
  const { reveal } = navigation;
  const history = selectionHistory(context, index, navigation, coordinator, select);
  coordinator.saveHistory = history.save;
  function button(text, action) {
    const el = document.createElement('button');
    el.type = 'button'; el.textContent = text; el.addEventListener('click', action);
    return el;
  }
  function section(heading, entries) {
    if (!entries.length) return;
    const h = document.createElement('h3'); h.textContent = heading;
    const list = document.createElement('ul');
    for (const entry of entries) {
      const li = document.createElement('li');
      if (entry.target && byId(entry.target)) {
        li.append(button(entry.text, () => select(entry.target)));
        if (entry.target !== coordinator.selected) byId(entry.target).setAttribute('data-viewer-related', 'true');
      } else li.textContent = entry.text;
      list.append(li);
    }
    details.append(h, list);
  }
  function copyIdentity(resource, heading) {
    const group = document.createElement('div'); group.className = 'copy-identity';
    const caption = document.createElement('p');
    caption.textContent = `${heading} · ${resource.mode} · ${resource.deposed == null ? 'current' : 'deposed: ' + resource.deposed} · State: ${resource.state || '(unnamed)'}`;
    const field = document.createElement('textarea');
    field.readOnly = true; field.rows = 2; field.value = resource.address;
    field.setAttribute('aria-label', `${heading}: native Terraform address`);
    const status = document.createElement('p'); status.setAttribute('role', 'status');
    const copy = button('Copy Terraform address', async () => {
      status.textContent = '';
      try {
        if (!window.isSecureContext || !navigator.clipboard?.writeText) throw new Error('Clipboard unavailable');
        await navigator.clipboard.writeText(resource.address);
        if (group.isConnected) status.textContent = 'Terraform address copied.';
      } catch {
        if (!group.isConnected) return;
        status.textContent = 'Automatic copy unavailable. Select the address and press Ctrl+C or Command+C to copy manually.';
        field.focus(); field.select();
      }
    });
    copy.setAttribute('aria-label', `Copy Terraform address: ${resource.address} · ${caption.textContent}`);
    group.append(caption, field, copy, status); details.append(group);
  }
  function select(id, restoring = false) {
    const el = byId(id);
    if (!el || !svg.contains(el)) return false;
    if (!restoring) history.before(id);
    coordinator.clear(); coordinator.selected = id;
    highlights.mark(el, 'selected');
    viewport.setAttribute('data-selection', 'true');
    const r = targets.get(id);
    const heading = document.createElement('h2'); heading.textContent = r ? label(r) : title(el);
    details.append(heading);
    if (relationshipEntries.has(id)) {
      for (const edge of relationshipEdges(id)) edge.setAttribute('data-viewer-edge', 'relationship');
      for (const link of links.filter(link => link.target === id && link.el.hasAttribute('data-relationship-select'))) link.el.setAttribute('data-viewer-related', 'true');
      section('Relationship endpoints', Array.from(el.querySelectorAll('[data-relationship-endpoint]')).map(link => ({ target: link.getAttribute('href').slice(1), text: link.textContent })));
      const state = el.closest('[data-state-id]')?.dataset.stateId || '';
      const sources = new Set(Array.from(el.querySelectorAll('[data-source-identity]')).map(row => row.dataset.sourceIdentity));
      for (const resource of targets.values()) {
        if (resource.state === state && sources.has(resource.identity)) copyIdentity(resource, 'Provenance resource');
      }
      const endpointIds = new Set(Array.from(el.querySelectorAll('[data-relationship-endpoint]')).map(link => link.getAttribute('href').slice(1)));
      for (const endpoint of endpointIds) {
        if (targets.has(endpoint)) copyIdentity(targets.get(endpoint), 'Endpoint resource');
      }
      reveal(el);
      if (!restoring) history.after();
      return true;
    }
    if (r) {
      const identity = document.createElement('p');
      identity.textContent = `${r.qualified}\nModule: ${r.module} · State: ${r.state || '(unnamed)'} · ${r.type}\n${title(el)}`;
      details.append(identity);
      copyIdentity(r, 'Selected resource');
    }
    const incoming = [], outgoing = [], undirected = [], provenance = [];
    for (const edge of edges) {
      const from = edge.dataset.source, to = edge.dataset.target;
      if (from !== id && to !== id) continue;
      const directed = edge.dataset.directionality === 'directed' || edge.dataset.crossState === 'true';
      const group = directed ? (from === id ? outgoing : incoming) : undirected;
      const direction = directed ? (from === id ? 'outgoing' : 'incoming') : 'undirected';
      edge.setAttribute('data-viewer-edge', direction);
      group.push({ target: from === id ? to : from, text: `${edge.dataset.edgeKind}: ${title(edge)}` });
    }
    section('Incoming rendered edges', incoming);
    section('Outgoing rendered edges', outgoing);
    section('Undirected rendered relationships', undirected);
    if (r?.relationship) {
      for (const entry of relationshipEntries.values()) {
        const state = entry.closest('[data-state-id]')?.dataset.stateId || '';
        if (state !== r.state || !Array.from(entry.querySelectorAll('[data-source-address]')).some(row => row.dataset.sourceIdentity === r.identity)) continue;
        for (const edge of relationshipEdges(entry.id)) edge.setAttribute('data-viewer-edge', 'provenance');
        for (const link of entry.querySelectorAll('[data-relationship-endpoint]')) provenance.push({ target: link.getAttribute('href').slice(1), text: link.textContent });
      }
    }
    section('Rendered relationships from this resource', provenance);
    const refs = [], inverse = [], navigation = [];
    for (const link of links) {
      const owner = link.el.closest('[data-viewer-target]');
      if (link.el.hasAttribute('data-reference-attribute')) {
        if (owner?.id === id) refs.push({ target: link.target, text: title(link.el) });
        if (link.target === id && owner) inverse.push({ target: owner.id, text: `${targets.has(owner.id) ? label(targets.get(owner.id)) : title(owner)}: ${title(link.el)}` });
      } else if (owner?.id === id && !link.el.closest('[data-entity-kind="synthetic"]')) {
        navigation.push({ target: link.target, text: link.el.textContent });
      }
    }
    section('Uses / reference targets', refs);
    section('Referenced by', inverse);
    section('Resource navigation', navigation);
    section('Unavailable / unresolved references', Array.from(el.querySelectorAll('[data-reference-status]')).filter(ref => ref.closest('[data-viewer-target]') === el).map(ref => ({ text: title(ref) + ' (' + ref.dataset.referenceStatus + ')' })));
    const memberships = [];
    for (const component of components) {
      const members = links.filter(link => component.contains(link.el));
      if (members.some(link => link.target === id)) memberships.push({ target: component.id, text: title(component) });
      if (component.id === id) section('Component members', members.map(link => ({ target: link.target, text: title(link.el) })));
    }
    section('Logical Component membership', memberships);
    if (details.children.length <= (r ? 3 : 1)) section('Relationships', [{ text: 'No rendered edges, references or component memberships in this view.' }]);
    for (const b of results.querySelectorAll('button')) b.setAttribute('aria-pressed', String(b.dataset.target === id));
    reveal(el);
    if (!restoring) history.after();
    return true;
  }

  return { select, button, clearHistory: history.clear };
}
