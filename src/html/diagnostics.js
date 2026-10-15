function diagnostics(context, index, navigation, selection, coordinator) {
  const { byId, svg } = context;
  const toggle = byId('hotspot-toggle'), list = byId('hotspot-list'), status = byId('hotspot-status');
  const source = byId('viewer-diagnostics');
  const data = source ? JSON.parse(source.textContent) : null;
  const available = data?.schema_version === 1 && data.available && data.scope === 'whole_diagram';
  toggle.disabled = !available;
  status.textContent = available
    ? `${data.total} collected findings; ${data.hotspots.length} groups shown; ${data.omitted} omitted. ${data.complete ? '' : 'Analysis incomplete. '}Bounds use whole-diagram coordinates. Crossings/proximity are advisory; overlap/minimum clearance are violations.`
    : 'Diagnostics unavailable. Generate with --format html --html-diagnostics.';
  const layer = document.createElementNS('http://www.w3.org/2000/svg', 'g');
  layer.setAttribute('data-hotspot-overlay', 'true');
  layer.setAttribute('aria-hidden', 'true');
  layer.style.pointerEvents = 'none';
  let active = false;
  function reset() {
    active = false;
    layer.remove();
    layer.replaceChildren();
    list.replaceChildren();
    list.hidden = true;
    toggle.setAttribute('aria-pressed', 'false');
    toggle.textContent = 'Show routing hotspots';
  }
  function endpoint(identity) {
    return index.resources.find(r => r.state === (identity?.state || '') &&
      (r.identity === identity?.id || r.address === identity?.id));
  }
  function show() {
    reset(); active = true;
    toggle.setAttribute('aria-pressed', 'true');
    toggle.textContent = 'Hide routing hotspots';
    list.hidden = false;
    svg.append(layer);
    for (const [i, hotspot] of data.hotspots.slice(0, 200).entries()) {
      const [x, y, width, height] = hotspot.bounds;
      const rect = document.createElementNS(svg.namespaceURI, 'rect');
      for (const [key, value] of Object.entries({ x: x - 6, y: y - 6, width: width + 12, height: height + 12, fill: 'none', stroke: hotspot.severity === 'violation' ? '#b91c1c' : '#b45309', 'stroke-width': 3, 'stroke-dasharray': '6 4' })) rect.setAttribute(key, value);
      layer.append(rect);
      const row = document.createElement('li');
      row.append(selection.button(`${hotspot.severity}: ${hotspot.kind} ×${hotspot.count} · (${x}, ${y}, ${width}, ${height})${hotspot.minimum_visible_gap === null ? '' : ' · minimum visible gap ' + hotspot.minimum_visible_gap + 'px'}`, () => {
        coordinator.clear();
        show();
        const region = layer.children[i];
        region.setAttribute('stroke-width', '6');
        navigation.reveal(region);
        list.children[i].querySelector('button').focus({ preventScroll: true });
      }));
      for (const path of hotspot.paths) {
        const description = document.createElement('details');
        const summary = document.createElement('summary');
        summary.textContent = `Path identity · representative segment ${path.segment ?? 'unavailable'}`;
        const text = document.createElement('p'); text.textContent = path.id;
        description.append(summary, text);
        for (const name of ['source', 'target']) {
          const target = endpoint(path.identity?.[name]);
          if (target && byId(target.id)) description.append(selection.button(`${name}: ${index.label(target)}`, () => selection.select(target.id)));
          else { const missing = document.createElement('p'); missing.textContent = `${name}: unavailable in this view`; description.append(missing); }
        }
        row.append(description);
      }
      list.append(row);
    }
  }
  toggle.addEventListener('click', () => { if (active) reset(); else if (available) show(); });
  return { reset };
}
