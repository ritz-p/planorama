function highlighting(svg, viewport) {
  function clear() {
    for (const el of svg.querySelectorAll('[data-viewer-selected], [data-viewer-related], [data-viewer-edge]')) {
      for (const key of ['data-viewer-selected', 'data-viewer-related', 'data-viewer-edge']) el.removeAttribute(key);
    }
    viewport.removeAttribute('data-selection');
  }
  function mark(el, kind, value = 'true') { el.setAttribute('data-viewer-' + kind, value); }

  return { clear, mark };
}
