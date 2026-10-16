function semanticDiagnostics(context, index, selection, coordinator) {
  const { byId } = context;
  const source = byId('viewer-semantic-diagnostics');
  const data = source ? JSON.parse(source.textContent) : null;
  const available = data?.schema_version === 1 && data.available;
  const toggle = byId('semantic-toggle'), panel = byId('semantic-panel');
  const status = byId('semantic-status'), list = byId('semantic-list');
  const search = byId('semantic-search'), category = byId('semantic-category');
  const key = r => JSON.stringify([r.state, r.address, r.deposed, r.mode]);
  const targets = new Map(index.resources.map(r => [key(r), r]));
  const limits = available
    ? `${data.total} selected findings; ${data.records.length} embedded; ${data.omitted} omitted. Supported rules only; cross-state mapping diagnostics are not included. Input completeness: ${data.input_complete === null ? 'unknown' : data.input_complete ? 'complete' : 'incomplete'}. ${data.unassociated_input_findings} unassociated input findings are excluded. Categories overlap; unknowns are informational.`
    : 'Semantic diagnostics unavailable. Generate with --format html --html-semantic-diagnostics.';
  toggle.disabled = !available;
  let active = false;
  function reset() {
    if (panel.contains(document.activeElement)) toggle.focus({ preventScroll: true });
    active = false;
    panel.hidden = true;
    list.replaceChildren();
    toggle.setAttribute('aria-expanded', 'false');
    toggle.textContent = 'Show semantic diagnostics';
    status.textContent = limits;
  }
  function render() {
    list.replaceChildren();
    const terms = search.value.toLowerCase().split(/\s+/).filter(Boolean);
    const matches = data.records.filter(record => {
      const text = [record.state, record.address, record.deposed, record.mode, record.attribute, record.reason, record.category, record.message].join(' ').toLowerCase();
      return (!category.value || category.value === record.category) && terms.every(term => text.includes(term));
    });
    const shown = matches.slice(0, 100);
    status.textContent = `${matches.length} matching embedded findings; ${shown.length} shown; ${matches.length - shown.length} hidden by list limit. ${limits}`;
    for (const record of shown) {
      const row = document.createElement('li');
      const label = `${record.state || '(unnamed)'}:${record.address} · ${record.mode} · ${record.deposed === null ? 'current' : 'deposed: ' + record.deposed}`;
      const target = targets.get(key(record));
      if (target && byId(target.id)) {
        row.append(selection.button(label, () => {
          selection.select(target.id);
          byId(target.id).focus({ preventScroll: true });
        }));
      } else {
        const identity = document.createElement('p');
        identity.textContent = `${label} · navigation unavailable in this view`;
        row.append(identity);
      }
      const detail = document.createElement('p');
      detail.textContent = `${record.category} / ${record.reason}: ${record.message}${record.attribute === null ? '' : ' · Attribute: ' + record.attribute}`;
      row.append(detail);
      list.append(row);
    }
  }
  toggle.addEventListener('click', () => {
    if (active) reset();
    else if (available) {
      coordinator.clear();
      active = true;
      panel.hidden = false;
      toggle.setAttribute('aria-expanded', 'true');
      toggle.textContent = 'Hide semantic diagnostics';
      render();
      search.focus();
    }
  });
  search.addEventListener('input', render);
  category.addEventListener('change', render);
  reset();
  return { reset };
}
