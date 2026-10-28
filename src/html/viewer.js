(() => {
  'use strict';
/* VIEWER_MODULES */
  const byId = id => {
    const el = document.getElementById(id);
    return el?.closest('[data-html-filtered]') ? null : el;
  };
  generationInformation(byId);
  const viewport = byId('viewport');
  const svg = viewport.querySelector('svg');
  const search = byId('search');
  const results = byId('results');
  const details = byId('selection');
  const context = { byId, viewport, svg, search, results, details };
  const index = metadata(svg, byId);
  const { resources, label, title, components } = index;
  const view = navigation(svg, viewport, byId);
  overview(context, view);
  const exporter = diagramExport(context);
  const highlights = highlighting(svg, viewport);
  const adapters = lifecycle();
  const coordinator = {
    selected: null,
    clear() {
      coordinator.saveHistory?.();
      coordinator.selected = null;
      adapters.reset();
      highlights.clear();
      details.replaceChildren();
      for (const b of results.querySelectorAll('button')) b.setAttribute('aria-pressed', 'false');
    }
  };
  const { select, button, clearHistory } = selection(context, index, view, highlights, coordinator);
  neighborhoodExport(context, index, coordinator, exporter);
  const clear = () => { clearHistory(); coordinator.clear(); };
  const { renderResults } = searching(context, index, { select, button }, coordinator);
  adapters.register(diagnostics(context, index, view, { select, button }, coordinator));
  adapters.register(semanticDiagnostics(context, index, { select, button }, coordinator));
  const impactView = impact(context, index, { select, button }, coordinator);
  adapters.register(impactView);
  actionFiltering(context, index, view, coordinator, renderResults);
  search.addEventListener('input', () => adapters.reset());
  const { fit, zoom, updateView } = view;
  const destinations = [...svg.querySelectorAll('[data-architecture], [data-annotation-section]')].filter(el => {
    if (el.hasAttribute('data-architecture') && !el.querySelector('[data-terraform-address]')) return false;
    const bounds = el.getBBox();
    return bounds.width > 0 && bounds.height > 0;
  });
  const sectionSelect = byId('section-destination');
  for (const [i, el] of destinations.entries()) {
    const state = el.closest('[data-state-id]')?.dataset.stateId;
    const name = el.dataset.annotationSection || 'Architecture';
    const option = document.createElement('option');
    option.value = String(i);
    option.textContent = state ? `${state}: ${name}` : name;
    sectionSelect.append(option);
  }
  sectionSelect.disabled = byId('section-go').disabled = destinations.length === 0;
  byId('section-go').addEventListener('click', () => {
    const destination = destinations[Number(sectionSelect.value)];
    if (!destination) return;
    coordinator.clear();
    view.reveal(destination);
    byId('section-status').textContent = `Showing ${sectionSelect.selectedOptions[0].textContent}`;
    viewport.focus({ preventScroll: true });
  });
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
  viewport.addEventListener('click', event => {
    if (view.wasDragged()) { event.preventDefault(); return; }
    const el = selectionTarget(event.target);
    if (el) { event.preventDefault(); select(el.id); }
  });
  viewport.addEventListener('keydown', event => {
    if (['Enter', ' '].includes(event.key)) {
      const el = selectionTarget(event.target);
      if (el) { event.preventDefault(); select(el.id); return; }
    }
    view.keydown(event);
  });
  document.addEventListener('keydown', event => { if (event.key === 'Escape') { event.preventDefault(); clear(); } });
  byId('clear').addEventListener('click', clear);
  byId('fit').addEventListener('click', fit);
  byId('zoom-in').addEventListener('click', () => zoom(.8));
  byId('zoom-out').addEventListener('click', () => zoom(1.25));
  renderResults(); updateView();
})();
