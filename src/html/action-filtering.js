function actionFiltering(context, index, view, coordinator, renderResults) {
  const { svg, byId } = context;
  const panel = byId('logical-filters'), controls = byId('logical-filter-controls');
  const actions = ['Create', 'Update', 'Delete', 'Replace', 'Read', 'Unchanged', 'Other'];
  const number = (el, name) => Number(el.getAttribute(name));
  const translation = el => {
    const matrix = el.transform.baseVal.consolidate()?.matrix;
    return [matrix?.e || 0, matrix?.f || 0];
  };
  const move = (el, x, y) => el.setAttribute('transform', `translate(${x} ${y})`);
  const rootHeight = number(svg, 'height');
  const areas = [...svg.querySelectorAll('[data-annotation-area]')].map(el => ({
    el, height: Number(el.dataset.measuredHeight),
    sections: [...el.querySelectorAll(':scope > [data-annotation-card]')].map(frame => {
      const section = frame.querySelector(':scope > [data-annotation-section]');
      return { frame, section,
        cards: [...section.querySelectorAll(':scope > [data-annotation-card]')].map(card => ({
          el: card, width: number(card.firstElementChild, 'width'), height: number(card.firstElementChild, 'height')
        })) };
    }),
    architecture: el.parentElement.querySelector(':scope > [data-architecture]'),
    document: el.ownerSVGElement
  }));
  for (const area of areas) {
    area.offset = translation(area.architecture);
    area.documentHeight = number(area.document, 'height');
    area.documentBox = area.document.getAttribute('viewBox').split(/\s+/).map(Number);
  }
  const states = [...svg.querySelectorAll('[data-state-id]')].map(el => ({
    el, area: areas.find(area => el.contains(area.el))
  }));
  const paths = [...svg.querySelectorAll('path[data-cross-state]')].map(el => ({
    el, original: el.getAttribute('d'), points: el.getAttribute('d').match(/-?\d+(?:\.\d+)?/g).map(Number),
    split: Number(el.dataset.sourcePoints), source: el.dataset.sourceState, target: el.dataset.targetState,
    marker: el.nextElementSibling?.matches('circle[data-endpoint]') ? el.nextElementSibling : null
  }));
  const groups = [];
  for (const area of areas) for (const section of area.sections) for (const card of section.cards) {
    const group = card.el.querySelector(':scope > [data-logical-group]');
    if (!group) continue;
    const sections = [...group.querySelectorAll(':scope > [data-action-section]')].map(el => ({
      el, action: el.dataset.actionSection, start: number(el.firstElementChild, 'y'),
      height: number(el.firstElementChild, 'height'), count: el.querySelectorAll('[data-placement="logical"]').length
    }));
    const fieldset = document.createElement('fieldset');
    fieldset.dataset.resourceType = group.dataset.resourceType;
    fieldset.dataset.mode = group.dataset.mode;
    const legend = document.createElement('legend');
    const state = group.closest('[data-state-id]')?.dataset.stateId;
    legend.textContent = `${state ? state + ': ' : ''}${group.dataset.resourceType} · ${group.dataset.mode} · ${group.dataset.providerSource}`;
    fieldset.append(legend);
    const entry = { card, sections, start: sections[0].start, enabled: new Set(actions.map(action => action.toLowerCase())) };
    for (const action of actions) {
      const key = action.toLowerCase(), section = sections.find(section => section.action === key);
      const label = document.createElement('label'), input = document.createElement('input');
      input.type = 'checkbox'; input.checked = true; input.dataset.action = key;
      input.disabled = !section;
      input.addEventListener('change', () => {
        if (input.checked) entry.enabled.add(key); else entry.enabled.delete(key);
        update();
      });
      label.append(input, document.createTextNode(`${action} (${section?.count || 0})`)); fieldset.append(label);
    }
    entry.status = document.createElement('p'); entry.status.setAttribute('role', 'status');
    fieldset.append(entry.status); controls.append(fieldset); groups.push(entry);
    entry.status.textContent = `${sections.reduce((sum, section) => sum + section.count, 0)} visible`;
  }
  panel.hidden = !groups.length;
  const links = index.links.map(link => ({ ...link, href: link.el.getAttribute('href'), title: null }));
  const hotspot = byId('hotspot-toggle'), hotspotStatus = byId('hotspot-status');
  const hotspotDisabled = hotspot.disabled;
  const hotspotDescription = hotspotStatus.textContent;
  function update() {
    coordinator.clearHistory();
    coordinator.clear();
    let hidden = 0;
    for (const group of groups) {
      let y = group.start, visible = 0, total = 0;
      for (const section of group.sections) {
        total += section.count;
        const shown = group.enabled.has(section.action);
        if (shown) {
          section.el.removeAttribute('data-html-filtered'); section.el.removeAttribute('display');
          if (y === section.start) section.el.removeAttribute('transform');
          else move(section.el, 0, y - section.start);
          y += section.height + 12; visible += section.count;
        } else {
          section.el.setAttribute('data-html-filtered', 'true'); section.el.setAttribute('display', 'none');
        }
      }
      hidden += total - visible;
      group.card.height = y + 16;
      group.status.textContent = `${visible} of ${total} visible${visible ? '' : ' · All rows hidden by HTML action filters'}`;
    }
    for (const area of areas) {
      let y = 160;
      for (const { frame, section, cards } of area.sections) {
        let x = 16, top = 48, rowHeight = 0;
        const width = Number(section.dataset.packingWidth);
        for (const card of cards) {
          if (x > 16 && x + card.width > width - 16) { x = 16; top += rowHeight + 20; rowHeight = 0; }
          move(card.el, x, top); card.el.firstElementChild.setAttribute('height', card.height);
          x += card.width + 20; rowHeight = Math.max(rowHeight, card.height);
        }
        const height = top + rowHeight + 16;
        move(frame, 40, y);
        frame.firstElementChild.setAttribute('height', height);
        section.firstElementChild.setAttribute('height', height);
        y += height + 20;
      }
      area.delta = y - 160 - 20 + 24 - area.height;
      move(area.architecture, area.offset[0], area.offset[1] + area.delta);
      if (area.document !== svg) {
        area.document.setAttribute('height', area.documentHeight + area.delta);
        const box = area.documentBox.slice(); box[3] += area.delta;
        area.document.setAttribute('viewBox', box.join(' '));
      }
    }
    let delta = 0;
    const offsets = new Map();
    for (const state of states) {
      if (delta) move(state.el, 0, delta); else state.el.removeAttribute('transform');
      offsets.set(state.el.dataset.stateId, delta + (state.area?.delta || 0));
      delta += state.area?.delta || 0;
    }
    if (!states.length) delta = areas[0]?.delta || 0;
    for (const path of paths) {
      const from = offsets.get(path.source) || 0, to = offsets.get(path.target) || 0;
      path.el.setAttribute('d', from || to ? path.points.reduce((parts, value, i) => {
        if (!(i % 2)) parts.push(`${i ? 'L' : 'M'} ${value} ${path.points[i + 1] + (i / 2 < path.split ? from : to)}`);
        return parts;
      }, []).join(' ') : path.original);
      if (path.marker) { if (from) move(path.marker, 0, from); else path.marker.removeAttribute('transform'); }
    }
    svg.setAttribute('height', rootHeight + delta);
    view.resize(rootHeight + delta);
    for (const link of links) {
      if (!byId(link.target)) {
        link.el.removeAttribute('href'); link.el.setAttribute('aria-disabled', 'true');
        if (!link.title) {
          link.title = document.createElementNS(svg.namespaceURI, 'title');
          link.title.textContent = `${index.title(link.el) || link.el.textContent} (hidden by HTML action filters)`;
          link.el.prepend(link.title);
        }
      } else {
        link.el.setAttribute('href', link.href); link.el.removeAttribute('aria-disabled');
        link.title?.remove(); link.title = null;
      }
    }
    hotspot.disabled = hotspotDisabled || hidden > 0;
    hotspotStatus.textContent = hidden ? 'Routing hotspots describe the generated layout. Restore all HTML action filters to inspect them.' : hotspotDescription;
    byId('logical-filter-status').textContent = `${index.resources.length - hidden} of ${index.resources.length} generated resources visible; ${hidden} hidden by HTML action filters. Excluded resources are not restored.`;
    renderResults();
  }
  byId('logical-filter-reset').addEventListener('click', () => {
    for (const group of groups) group.enabled = new Set(actions.map(action => action.toLowerCase()));
    for (const input of controls.querySelectorAll('input')) input.checked = true;
    update();
  });
}
