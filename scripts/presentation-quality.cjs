const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const {Driver} = require('../tests/browser/webdriver.cjs');

function inspectPresentation() {
  const tolerance = 1;
  const pairLimit = 2000000;
  const findingLimit = 20000;
  const scopeSelector = '[data-architecture-id], [data-annotation-card], [data-route-entry]';
  const badgeSelector = '[data-service-badge], [data-import-badge] rect, g[data-external] > rect, [data-relationship-entry] rect';
  const frames = new Map();
  const items = [];
  const findings = [];
  let comparisons = 0;
  let complete = true;
  const box = (element, inset = 0, verticalInset = inset, b = element.getBBox()) => {
    const m = element.getCTM();
    if (!m) throw new Error('Missing SVG transform');
    const points = [[b.x + inset, b.y + verticalInset], [b.x + b.width - inset, b.y + verticalInset],
      [b.x + inset, b.y + b.height - verticalInset], [b.x + b.width - inset, b.y + b.height - verticalInset]]
      .map(([x, y]) => new DOMPoint(x, y).matrixTransform(m));
    const x = Math.min(...points.map(p => p.x)), y = Math.min(...points.map(p => p.y));
    return {x, y, width: Math.max(...points.map(p => p.x)) - x, height: Math.max(...points.map(p => p.y)) - y};
  };
  const stroke = element => getComputedStyle(element).stroke === 'none' ? 0 : parseFloat(getComputedStyle(element).strokeWidth) / 2;
  const identity = element => {
    const resource = element.closest('[data-terraform-address], [data-source-address], [data-architecture-id]');
    const section = element.closest('[data-action-section], [data-annotation-section], [data-route-entry]');
    return {state: element.closest('[data-state-id]')?.getAttribute('data-state-id') || null,
      resource: resource?.getAttribute('data-terraform-address') || resource?.getAttribute('data-source-address') || resource?.getAttribute('data-architecture-id') || null,
      group: element.closest('[data-resource-type]')?.getAttribute('data-resource-type') || null,
      relationship: element.closest('[data-relationship-entry]')?.getAttribute('data-relationship-entry') || null,
      section: section?.getAttribute('data-action-section') || section?.getAttribute('data-annotation-section') || section?.getAttribute('data-route-entry') || null};
  };
  const add = (element, kind) => {
    const item = {element, kind, bounds: box(element, -stroke(element)), ...identity(element)};
    items.push(item);
    return item;
  };
  for (const rect of document.querySelectorAll('rect')) {
    if (rect.closest('defs, [data-relationship-marker], [data-role-shape], [data-hotspot-overlay]')) continue;
    if (rect.matches(badgeSelector)) {
      add(rect, 'badge');
    } else if (rect.closest(scopeSelector) || rect.parentElement.matches('[data-container], [data-member-cluster], [data-route-table]')) {
      const frame = add(rect, 'frame');
      frame.content = box(rect, stroke(rect));
      frames.set(rect.parentElement, frame);
    }
  }
  const linkedFrames = new Map();
  for (const [owner, frame] of frames) {
    for (const attr of ['data-container', 'data-member-cluster', 'data-route-table']) {
      if (owner.hasAttribute(attr)) linkedFrames.set(owner.getAttribute(attr), frame);
    }
  }
  const enclosing = element => {
    for (let owner = element.parentElement; owner; owner = owner.parentElement) {
      if (frames.has(owner)) return frames.get(owner);
      if (linkedFrames.has(owner.id)) return linkedFrames.get(owner.id);
    }
    return null;
  };
  const badges = items.filter(item => item.kind === 'badge');
  const badgeElements = new Map(badges.map(b => [b.element, b]));
  const badgeFor = element => {
    if (element.closest('[data-service-name]')) {
      let previous = element.closest('text').previousElementSibling;
      while (previous?.matches('[data-service-name]')) previous = previous.previousElementSibling;
      return badgeElements.get(previous);
    }
    const owner = element.closest('[data-import-badge], g[data-external], [data-relationship-entry]');
    if (!owner) return null;
    const firstText = owner.querySelector('text');
    return firstText?.contains(element) ? badgeElements.get(owner.querySelector('rect')) : null;
  };
  for (const text of document.querySelectorAll('text')) {
    if (!text.closest(scopeSelector) || text.closest('defs, [data-relationship-marker], [data-hotspot-overlay]')) continue;
    const lines = text.querySelectorAll('tspan');
    for (const line of lines.length ? lines : [text]) {
      if (!line.textContent.trim()) continue;
      const item = add(line, text.matches('[data-resource-type-heading], [data-action-heading]') ? 'heading' : 'text');
      item.text = line.textContent;
      item.badge = badgeFor(line);
    }
  }
  const contains = (a, b) => b.x >= a.x - tolerance && b.y >= a.y - tolerance &&
    b.x + b.width <= a.x + a.width + tolerance && b.y + b.height <= a.y + a.height + tolerance;
  const intersects = (a, b) => Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x) > tolerance &&
    Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y) > tolerance;
  const describe = item => ({kind: item.kind, state: item.state, resource: item.resource, group: item.group, relationship: item.relationship,
    section: item.section, bounds: item.bounds, ...(item.content ? {content_bounds: item.content} : {}), ...(item.text ? {text: item.text} : {})});
  const record = (category, item, peer) => {
    if (findings.length === findingLimit) { complete = false; return; }
    findings.push({category, element: describe(item), peer: describe(peer)});
  };
  for (const item of items) {
    if (item.kind === 'frame') continue;
    const frame = enclosing(item.element);
    if (frame && !contains(frame.content, item.bounds)) record('content-overflow', item, frame);
    const header = item.element.closest('[data-presentation-header]');
    if (header && !item.element.closest('[data-routes], [data-cluster-members]')) {
      const [x, y, width, height] = header.getAttribute('data-presentation-header').split(' ').map(Number);
      const bounds = box(header, 0, 0, {x, y, width, height});
      if (!contains(bounds, item.bounds)) record('header-overflow', item, {...identity(header), kind: 'header-content', bounds});
    }
    if (item.badge) {
      const element = item.badge.element;
      const [horizontal = 0, vertical = 0] = (element.getAttribute('data-presentation-padding') || '').split(' ').map(Number);
      const content = box(element, horizontal + stroke(element), vertical + stroke(element));
      if (!contains(content, item.bounds)) record('badge-text-overflow', item, {...item.badge, content});
    }
  }
  items.sort((a, b) => a.bounds.y - b.bounds.y);
  let active = [];
  outer: for (const item of items) {
    active = active.filter(peer => peer.bounds.y + peer.bounds.height > item.bounds.y + tolerance);
    for (const peer of active) {
      if (++comparisons > pairLimit) { complete = false; break outer; }
      if (item.kind === 'frame' || peer.kind === 'frame') {
        const frame = item.kind === 'frame' ? item : peer;
        const badge = item.kind === 'frame' ? peer : item;
        if (badge.kind === 'badge' && intersects(frame.bounds, badge.bounds) &&
            !contains(frame.content, badge.bounds))
          record('badge-frame-intersection', badge, frame);
      } else if (item.badge !== peer && peer.badge !== item && intersects(item.bounds, peer.bounds)) {
        record('overlap', item, peer);
      }
    }
    active.push(item);
  }
  return {schema_version: 1, measurement: 'browser-observed SVG bounding boxes', browser: navigator.userAgent, units: 'SVG viewport px',
    policy: {tolerance, fonts: [...new Set([...document.querySelectorAll('text')].map(e => getComputedStyle(e).fontFamily))]},
    limits: {pair_limit: pairLimit, finding_limit: findingLimit, comparisons, complete},
    counts: {frames: frames.size, badges: badges.length, text: items.filter(i => ['text', 'heading'].includes(i.kind)).length}, findings};
}

async function checkFiles(files, output) {
  fs.mkdirSync(output, {recursive: true});
  const driver = await Driver.start();
  const results = [];
  try {
    for (const file of files) {
      try {
        driver.get(pathToFileURL(path.resolve(file)).href);
        const report = driver.execute_script(`return (${inspectPresentation.toString()})();`);
        if (!report.counts.text || !report.counts.frames) throw new Error('No supported presentation geometry');
        results.push({file, status: report.limits.complete && !report.findings.length ? 'passed' : 'failed', ...report});
      } catch (error) { results.push({file, status: 'error', error: error.message}); }
    }
  } finally { await driver.quit(); }
  fs.writeFileSync(path.join(output, 'presentation.json'), JSON.stringify(results, null, 2) + '\n');
  fs.writeFileSync(path.join(output, 'presentation.md'), '# Presentation quality\n\n' + results.map(r =>
    `- ${r.file}: ${r.status} (${r.findings?.length ?? '?'} findings)${r.error ? `: ${r.error}` : ''}`).join('\n') + '\n');
  return results.length > 0 && results.every(r => r.status === 'passed');
}

if (require.main === module) {
  const [output, ...args] = process.argv.slice(2);
  const files = args.flatMap(arg => arg === '--maintained' ? fs.readFileSync('scripts/generated-svgs.txt', 'utf8').trim().split(/\r?\n/) : [arg]);
  if (!output || !files.length) { console.error('Usage: node scripts/presentation-quality.cjs OUTPUT SVG...'); process.exitCode = 2; }
  else checkFiles(files, output).then(ok => { process.exitCode = ok ? 0 : 1; }).catch(error => { console.error(error); process.exitCode = 2; });
}
module.exports = {inspectPresentation, checkFiles};
