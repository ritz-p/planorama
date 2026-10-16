function selectionHistory(context, index, view, coordinator, select) {
  const { byId, svg } = context;
  const back = byId('selection-back'), forward = byId('selection-forward'), status = byId('history-status');
  let entries = [], cursor = -1;
  function identity(id) {
    const el = byId(id);
    if (!el || !svg.contains(el)) return null;
    const resource = index.targets.get(id);
    return JSON.stringify(resource ? ['resource', resource.state, resource.identity, id] :
      [index.relationshipEntries.has(id) ? 'relationship' : 'diagram', el.closest('[data-state-id]')?.dataset.stateId || '', id]);
  }
  function destination(step) {
    for (let i = cursor + step; i >= 0 && i < entries.length; i += step) {
      if (identity(entries[i].id) === entries[i].identity) return i;
    }
    return -1;
  }
  function update(message) {
    back.disabled = destination(-1) < 0; forward.disabled = destination(1) < 0;
    status.textContent = message || (entries.length ? `Selection ${cursor + 1} of ${entries.length}` : 'No selection history');
  }
  function save() {
    if (cursor >= 0 && entries[cursor].id === coordinator.selected) entries[cursor].box = view.snapshot();
  }
  function before(id) {
    save();
    const key = identity(id);
    if (entries[cursor]?.identity === key) return;
    entries.splice(cursor + 1);
    entries.push({ id, identity: key, box: view.snapshot() });
    if (entries.length > 100) entries.shift();
    cursor = entries.length - 1;
  }
  function after() { save(); update(); }
  function move(step) {
    save();
    const next = destination(step);
    if (next < 0) { update('No available selection in that direction.'); return; }
    const entry = entries[next];
    if (!select(entry.id, true)) { update('Selection is no longer available.'); return; }
    cursor = next;
    view.restore(entry.box);
    const el = byId(entry.id);
    if (!el.hasAttribute('tabindex')) el.setAttribute('tabindex', '-1');
    el.focus({ preventScroll: true });
    update();
  }
  function clear() { entries = []; cursor = -1; update(); }
  back.addEventListener('click', () => move(-1));
  forward.addEventListener('click', () => move(1));
  update();
  return { before, after, clear, save };
}
