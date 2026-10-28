function overview({ svg, byId }, view) {
  const panel = byId('overview');
  function element(name, attributes) {
    const el = document.createElementNS(svg.namespaceURI, name);
    for (const [key, value] of Object.entries(attributes)) el.setAttribute(key, value);
    return el;
  }
  const map = element('svg', { id: 'overview-map', tabindex: '0', role: 'group', 'aria-label': 'Diagram overview navigation', 'aria-describedby': 'overview-help overview-status', preserveAspectRatio: 'xMidYMid meet' });
  const shapes = element('g', { id: 'overview-shapes', 'aria-hidden': 'true' });
  const frame = element('rect', { id: 'overview-frame', 'aria-hidden': 'true' });
  map.append(shapes, frame); byId('overview-content').append(map); panel.hidden = false;
  const limit = 256;
  let dirty = true, pending = false, pointer = null;
  panel.open = !matchMedia('(max-width: 700px)').matches;
  function rect(el, box) {
    for (const [i, name] of ['x', 'y', 'width', 'height'].entries()) el.setAttribute(name, box[i]);
  }
  function rebuild() {
    map.setAttribute('viewBox', view.bounds().join(' '));
    const candidates = [...svg.querySelectorAll('[data-deployment-scope] > rect, [data-terraform-address] > rect:first-of-type, [data-annotation-card] > rect:first-of-type')]
      .filter(el => !el.closest('[data-html-filtered]'));
    const count = Math.min(limit, candidates.length), fragment = document.createDocumentFragment();
    const inverse = svg.getScreenCTM().inverse();
    for (let i = 0; i < count; i++) {
      const source = candidates[Math.floor(i * candidates.length / count)];
      const bounds = source.getBBox(), matrix = inverse.multiply(source.getScreenCTM());
      const points = [[bounds.x, bounds.y], [bounds.x + bounds.width, bounds.y], [bounds.x, bounds.y + bounds.height], [bounds.x + bounds.width, bounds.y + bounds.height]]
        .map(([x, y]) => new DOMPoint(x, y).matrixTransform(matrix));
      const xs = points.map(p => p.x), ys = points.map(p => p.y);
      const box = [Math.min(...xs), Math.min(...ys), Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)];
      if (!box.every(Number.isFinite) || box[2] <= 0 || box[3] <= 0) continue;
      const shape = document.createElementNS(svg.namespaceURI, 'rect');
      rect(shape, box); fragment.append(shape);
    }
    shapes.replaceChildren(fragment);
    byId('overview-status').textContent = `${shapes.childElementCount} of ${candidates.length} scope/card outlines shown${candidates.length > limit ? ' (sampled across the diagram; some omitted)' : ''}. Labels, rows and relationships are omitted. This is navigation, not a resource count.`;
    dirty = false;
  }
  function schedule(changed = false) {
    dirty ||= changed;
    if (pending || !panel.open) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      if (!panel.open) return;
      if (dirty) rebuild();
      rect(frame, view.snapshot());
    });
  }
  view.subscribe(schedule);
  panel.addEventListener('toggle', () => { pointer = null; schedule(); });
  function move(event) {
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(map.getScreenCTM().inverse());
    const [x, y, width, height] = view.bounds();
    view.center(Math.max(x, Math.min(x + width, point.x)), Math.max(y, Math.min(y + height, point.y)));
  }
  map.addEventListener('pointerdown', event => {
    if (event.button !== 0 || pointer !== null) return;
    event.preventDefault(); map.focus({ preventScroll: true });
    pointer = event.pointerId; map.setPointerCapture(pointer); move(event);
  });
  map.addEventListener('pointermove', event => { if (pointer === event.pointerId) move(event); });
  map.addEventListener('pointerup', event => {
    if (pointer !== event.pointerId) return;
    move(event); pointer = null; map.releasePointerCapture(event.pointerId);
  });
  for (const name of ['pointercancel', 'lostpointercapture']) map.addEventListener(name, () => { pointer = null; });
  map.addEventListener('keydown', event => {
    if (event.key === 'Escape') {
      event.preventDefault(); event.stopPropagation(); panel.open = false; panel.querySelector('summary').focus({ preventScroll: true });
    } else view.keydown(event);
  });
  schedule();
}
