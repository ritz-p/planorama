(() => {
  'use strict';
  const byId = id => document.getElementById(id);
  const viewport = byId('viewport');
  const svg = viewport.querySelector('svg');
  const search = byId('search');
  const results = byId('results');
  const details = byId('selection');
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
        const display = source.address + (source.deposed_key === null ? '' : ` (deposed: ${source.deposed_key})`);
        const row = rows.find(row => row.dataset.sourceAddress === source.address && row.querySelector('title')?.textContent === display);
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
  let selected = null;
  let box = [svg.viewBox.baseVal.x, svg.viewBox.baseVal.y, svg.viewBox.baseVal.width, svg.viewBox.baseVal.height];
  const original = box.slice();
  function updateView() {
    svg.setAttribute('viewBox', box.join(' '));
    byId('zoom-level').textContent = Math.round(original[2] / box[2] * 100) + '%';
  }
  function fit() { box = original.slice(); updateView(); }
  function zoom(factor) {
    const width = Math.max(40, Math.min(original[2] * 4, box[2] * factor));
    const ratio = width / box[2];
    box = [box[0] + box[2] * (1 - ratio) / 2, box[1] + box[3] * (1 - ratio) / 2, width, box[3] * ratio];
    updateView();
  }
  function reveal(el) {
    const bounds = el.getBBox();
    const matrix = svg.getScreenCTM().inverse().multiply(el.getScreenCTM());
    const points = [[bounds.x, bounds.y], [bounds.x + bounds.width, bounds.y + bounds.height]].map(([x, y]) => new DOMPoint(x, y).matrixTransform(matrix));
    const width = Math.max(300, Math.abs(points[1].x - points[0].x) + 100);
    const height = Math.max(200, Math.abs(points[1].y - points[0].y) + 100);
    const aspect = viewport.clientWidth / Math.max(1, viewport.clientHeight);
    const w = Math.max(width, height * aspect);
    const h = w / aspect;
    box = [(points[0].x + points[1].x - w) / 2, (points[0].y + points[1].y - h) / 2, w, h];
    updateView();
  }
  function clear() {
    selected = null;
    for (const el of svg.querySelectorAll('[data-viewer-selected], [data-viewer-related], [data-viewer-edge]')) {
      for (const key of ['data-viewer-selected', 'data-viewer-related', 'data-viewer-edge']) el.removeAttribute(key);
    }
    viewport.removeAttribute('data-selection');
    details.replaceChildren();
    for (const button of results.querySelectorAll('button')) button.setAttribute('aria-pressed', 'false');
  }
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
        if (entry.target !== selected) byId(entry.target).setAttribute('data-viewer-related', 'true');
      } else li.textContent = entry.text;
      list.append(li);
    }
    details.append(h, list);
  }
  function select(id) {
    const el = byId(id);
    if (!el || !svg.contains(el)) return;
    clear(); selected = id;
    el.setAttribute('data-viewer-selected', 'true');
    viewport.setAttribute('data-selection', 'true');
    const r = targets.get(id);
    const heading = document.createElement('h2'); heading.textContent = r ? label(r) : title(el);
    details.append(heading);
    if (r) {
      const identity = document.createElement('p');
      identity.textContent = `${r.qualified}\nModule: ${r.module} · State: ${r.state || '(unnamed)'} · ${r.type}\n${title(el)}`;
      details.append(identity);
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
      const scope = el.closest('[data-state-id]') || svg;
      const metadata = scope.querySelector('metadata[id$="architecture-relationships"]');
      for (const relation of JSON.parse(metadata.textContent)) {
        if (!relation.provenance.some(p => p.kind === 'resource' && p.address === r.address && p.deposed_key === r.deposed)) continue;
        const from = resources.find(other => other.state === r.state && other.identity === relation.from);
        const to = resources.find(other => other.state === r.state && other.identity === relation.to);
        const source = relation.provenance.find(p => p.kind === 'resource' && p.address === r.address && p.deposed_key === r.deposed);
        const expectedTitle = `${from?.address} ${relation.directionality === 'directed' ? '→' : '↔'} ${to?.address} (${relation.kind}; ${source.operation}: ${r.address})`;
        for (const edge of edges.filter(edge => edge.dataset.source === from?.id && edge.dataset.target === to?.id && edge.dataset.edgeKind === relation.kind && title(edge).startsWith(expectedTitle))) {
          edge.setAttribute('data-viewer-edge', 'provenance');
        }
        if (from) provenance.push({ target: from.id, text: `${relation.kind} ${relation.directionality}: ${label(from)} ${relation.directionality === 'directed' ? '→' : '↔'} ${to ? label(to) : relation.to}` });
        if (to) provenance.push({ target: to.id, text: `Endpoint: ${label(to)}` });
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
    if (details.children.length <= (r ? 2 : 1)) section('Relationships', [{ text: 'No rendered edges, references or component memberships in this view.' }]);
    for (const b of results.querySelectorAll('button')) b.setAttribute('aria-pressed', String(b.dataset.target === id));
    reveal(el);
  }
  function renderResults() {
    const terms = search.value.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
    const matched = resources.filter(r => terms.every(term => [label(r), r.qualified, r.state + ':' + r.qualified, r.identity, r.module, r.state, r.type, r.mode, r.deposed || ''].join(' ').toLocaleLowerCase().includes(term)));
    results.replaceChildren();
    for (const r of matched) {
      const b = button(label(r), () => select(r.id));
      b.dataset.target = r.id; b.setAttribute('aria-pressed', String(selected === r.id)); results.append(b);
    }
    byId('result-count').textContent = `${matched.length} of ${resources.length} resources`;
  }
  for (const r of resources) {
    const el = byId(r.id); el.setAttribute('data-viewer-target', 'true');
    el.setAttribute('tabindex', '0'); el.setAttribute('role', 'button'); el.setAttribute('aria-label', label(r));
  }
  for (const el of components) { el.setAttribute('tabindex', '0'); el.setAttribute('role', 'button'); el.setAttribute('aria-label', title(el)); }
  svg.setAttribute('role', 'group');
  for (const nested of svg.querySelectorAll('svg')) nested.setAttribute('role', 'group');
  const selectionTarget = target => {
    const anchor = target.closest('a[href^="#"]');
    if (anchor) return byId(anchor.getAttribute('href').slice(1));
    return target.closest('[data-viewer-target], [data-entity-kind="synthetic"][id]');
  };
  let dragged = false;
  viewport.addEventListener('click', event => {
    if (dragged) { event.preventDefault(); return; }
    const el = selectionTarget(event.target);
    if (el) { event.preventDefault(); select(el.id); }
  });
  viewport.addEventListener('keydown', event => {
    if (['Enter', ' '].includes(event.key)) {
      const el = selectionTarget(event.target);
      if (el) { event.preventDefault(); select(el.id); return; }
    }
    const shifts = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (shifts[event.key]) { event.preventDefault(); box[0] += shifts[event.key][0] * box[2] / 10; box[1] += shifts[event.key][1] * box[3] / 10; updateView(); }
    if (event.key === '+' || event.key === '=') { event.preventDefault(); zoom(.8); }
    if (event.key === '-') { event.preventDefault(); zoom(1.25); }
    if (event.key.toLowerCase() === 'f') { event.preventDefault(); fit(); }
  });
  document.addEventListener('keydown', event => { if (event.key === 'Escape') { event.preventDefault(); clear(); } });
  search.addEventListener('input', renderResults);
  search.addEventListener('keydown', event => {
    if (event.key === 'ArrowDown') { event.preventDefault(); results.querySelector('button')?.focus(); }
    if (event.key === 'Enter') { event.preventDefault(); results.querySelector('button')?.click(); }
  });
  results.addEventListener('keydown', event => {
    if (event.key === 'ArrowDown') { event.preventDefault(); event.target.nextElementSibling?.focus(); }
    if (event.key === 'ArrowUp') { event.preventDefault(); (event.target.previousElementSibling || search).focus(); }
    if (event.key === 'Home') { event.preventDefault(); results.firstElementChild?.focus(); }
    if (event.key === 'End') { event.preventDefault(); results.lastElementChild?.focus(); }
  });
  let drag = null;
  viewport.addEventListener('pointerdown', event => {
    if (event.button !== 0) return;
    dragged = false;
    drag = { x: event.clientX, y: event.clientY, box: box.slice(), matrix: svg.getScreenCTM().inverse(), id: event.pointerId };
  });
  viewport.addEventListener('pointermove', event => {
    if (!drag || drag.id !== event.pointerId) return;
    if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 4 && !dragged) return;
    dragged = true; viewport.setPointerCapture(event.pointerId);
    const start = new DOMPoint(drag.x, drag.y).matrixTransform(drag.matrix);
    const end = new DOMPoint(event.clientX, event.clientY).matrixTransform(drag.matrix);
    box = [drag.box[0] + start.x - end.x, drag.box[1] + start.y - end.y, drag.box[2], drag.box[3]]; updateView();
  });
  document.addEventListener('pointerup', () => { drag = null; });
  viewport.addEventListener('pointercancel', () => { drag = null; });
  viewport.addEventListener('wheel', event => {
    event.preventDefault();
    if (event.ctrlKey || event.metaKey) zoom(event.deltaY > 0 ? 1.1 : 1 / 1.1);
    else { const scale = 1 / svg.getScreenCTM().a; box[0] += event.deltaX * scale; box[1] += event.deltaY * scale; updateView(); }
  }, { passive: false });
  byId('clear').addEventListener('click', clear);
  byId('fit').addEventListener('click', fit);
  byId('zoom-in').addEventListener('click', () => zoom(.8));
  byId('zoom-out').addEventListener('click', () => zoom(1.25));
  renderResults(); updateView();
})();
