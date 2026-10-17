function diagramExport(context) {
  const { svg, byId } = context;
  const status = byId('export-status');
  function standalone(source, description) {
    const clone = source.cloneNode(true);
    clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    clone.setAttribute('overflow', 'hidden');
    const box = clone.getAttribute('viewBox').trim().split(/[ ,]+/).map(Number);
    if (box.length !== 4 || !box.every(Number.isFinite) || box[2] <= 0 || box[3] <= 0) throw new Error('Invalid export viewport');
    clone.setAttribute('width', box[2]); clone.setAttribute('height', box[3]);
    for (const el of clone.querySelectorAll('[data-hotspot-overlay], script, foreignObject')) el.remove();
    const ids = new Set();
    for (const el of [clone, ...clone.querySelectorAll('*')]) {
      if (el.id) {
        if (ids.has(el.id)) throw new Error('Duplicate SVG identity');
        ids.add(el.id);
      }
      for (const attr of [...el.attributes]) {
        if (/^on/i.test(attr.name) || /^data-(viewer|impact)-/.test(attr.name) || ['tabindex', 'role', 'aria-pressed'].includes(attr.name)) el.removeAttributeNode(attr);
      }
    }
    for (const el of [clone, ...clone.querySelectorAll('*')]) {
      if (el.localName === 'style' && /@import|url\s*\(/i.test(el.textContent)) throw new Error('Unsupported SVG stylesheet');
      for (const attr of [...el.attributes]) {
        if (['aria-labelledby', 'aria-describedby'].includes(attr.name) && attr.value.split(/\s+/).some(id => !ids.has(id))) el.removeAttributeNode(attr);
        if (attr.localName === 'href') {
          const value = attr.value;
          if (!value.startsWith('#') || !ids.has(value.slice(1))) {
            if (el.localName !== 'a') throw new Error('Unavailable SVG asset');
            el.removeAttributeNode(attr);
            const title = document.createElementNS(svg.namespaceURI, 'title');
            title.textContent = 'Destination unavailable in this export'; el.append(title);
          }
        }
        for (const match of attr.value.matchAll(/url\(\s*['"]?([^)'"\s]+)['"]?\s*\)/g)) {
          if (!match[1].startsWith('#') || !ids.has(match[1].slice(1))) throw new Error('Unavailable SVG definition');
        }
      }
    }
    const desc = document.createElementNS(svg.namespaceURI, 'desc');
    desc.textContent = description; clone.prepend(desc);
    const text = new XMLSerializer().serializeToString(clone);
    if (new DOMParser().parseFromString(text, 'image/svg+xml').querySelector('parsererror')) throw new Error('Invalid SVG document');
    return new Blob([text], { type: 'image/svg+xml;charset=utf-8' });
  }
  function download(source, description, filename) {
    let url, anchor;
    try {
      anchor = document.createElement('a');
      if (!('download' in anchor) || typeof URL.createObjectURL !== 'function') throw new Error('SVG downloads are unsupported in this browser');
      url = URL.createObjectURL(standalone(source, description));
      anchor.href = url; anchor.download = filename;
      document.body.append(anchor); anchor.click();
      status.textContent = 'SVG download requested. ' + description + ' If no file appears, check browser download permissions.';
    } catch (error) {
      status.textContent = 'Export failed: ' + error.message;
    } finally {
      anchor?.remove();
      if (url) setTimeout(() => URL.revokeObjectURL(url), 60000);
    }
  }
  byId('export-viewport').addEventListener('click', () => download(svg,
    'Current viewport; content outside this viewBox is intentionally cropped. Selection and diagnostic overlays are omitted.', 'planorama-viewport.svg'));
  return { download };
}
