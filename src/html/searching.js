function searching(context, index, selection, coordinator) {
  const { search, results, byId } = context;
  const { resources, label } = index;
  const { button, select } = selection;
  function renderResults() {
    const terms = search.value.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
    const matched = resources.filter(r => terms.every(term => [label(r), r.qualified, r.state + ':' + r.qualified, r.identity, r.module, r.state, r.type, r.mode, r.deposed || ''].join(' ').toLocaleLowerCase().includes(term)));
    results.replaceChildren();
    for (const r of matched) {
      const b = button(label(r), () => select(r.id));
      b.dataset.target = r.id; b.setAttribute('aria-pressed', String(coordinator.selected === r.id)); results.append(b);
    }
    byId('result-count').textContent = `${matched.length} of ${resources.length} resources`;
  }
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
