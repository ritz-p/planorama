function navigation(svg, viewport, byId) {
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
  let drag = null;
  let dragged = false;
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
  function keydown(event) {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const shifts = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (shifts[event.key]) { event.preventDefault(); box[0] += shifts[event.key][0] * box[2] / 10; box[1] += shifts[event.key][1] * box[3] / 10; updateView(); }
    if (event.key === '+' || event.key === '=') { event.preventDefault(); zoom(.8); }
    if (event.key === '-') { event.preventDefault(); zoom(1.25); }
    if (event.key.toLowerCase() === 'f') { event.preventDefault(); fit(); }
  }

  function restore(saved) {
    box = saved.slice(); updateView();
  }
  function resize(height) {
    original[3] = height;
    fit();
  }
  return { fit, zoom, reveal, updateView, keydown, resize, snapshot: () => box.slice(), restore, wasDragged: () => dragged };
}
