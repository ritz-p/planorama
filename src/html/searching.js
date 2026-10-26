function searching(context, index, selection, coordinator) {
  const { search, results, byId } = context;
  const { resources, label } = index;
  const { button, select } = selection;
  const action = byId('search-action'), state = byId('search-state'), type = byId('search-type');
  const typeKey = resource => JSON.stringify([resource.provider || null, resource.type]);
  function options(control, entries) {
    for (const [value, text] of entries) {
      const option = document.createElement('option'); option.value = value; option.textContent = text;
      control.append(option);
    }
  }
  options(action, ['Create', 'Update', 'Delete', 'Replace', 'Read', 'Unchanged', 'Other'].map(label => [label.toLowerCase(), label]));
  options(state, [...new Set(resources.map(r => r.state))].sort().map(value => [JSON.stringify(value), value || '(unnamed)']));
  options(type, [...new Map(resources.map(r => [typeKey(r), `${r.type} · ${r.provider || 'unknown provider source'}`]))].sort((a, b) => a[1].localeCompare(b[1])));
  function renderResults() {
    const terms = search.value.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
    const focused = results.contains(document.activeElement);
    const visible = resources.filter(r => byId(r.id));
    const matched = visible.filter(r => (!action.value || r.action === action.value) && (!state.value || JSON.stringify(r.state) === state.value) && (!type.value || typeKey(r) === type.value) && terms.every(term => [label(r), r.qualified, r.state + ':' + r.qualified, r.identity, r.module, r.state, r.type, r.provider || '', r.mode, r.deposed || ''].join(' ').toLocaleLowerCase().includes(term)));
    results.replaceChildren();
    for (const r of matched) {
      const b = button(label(r), () => select(r.id));
      b.dataset.target = r.id; b.setAttribute('aria-pressed', String(coordinator.selected === r.id)); results.append(b);
    }
    byId('result-count').textContent = `${matched.length} of ${resources.length} resources`;
    if (visible.length !== resources.length) byId('result-count').textContent += `; ${resources.length - visible.length} hidden by HTML action filters`;
    if (!matched.length) byId('result-count').textContent += '. No visible resources match this search.';
    if (focused) (results.firstElementChild || search).focus();
  }
  for (const control of [action, state, type]) control.addEventListener('change', () => {
    search.dispatchEvent(new Event('input'));
  });
  byId('search-reset').addEventListener('click', () => {
    search.value = ''; action.value = ''; state.value = ''; type.value = '';
    search.dispatchEvent(new Event('input')); search.focus();
  });
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

  return { renderResults };
}
