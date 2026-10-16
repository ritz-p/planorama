const { readFileSync, statSync, existsSync } = require('node:fs');
const { resolve, join } = require('node:path');
const { pathToFileURL } = require('node:url');
const { Driver } = require('../browser/webdriver.cjs');

async function main() {
  const directory = resolve(__dirname, '../../examples/multi-account');
  const driver = await Driver.start();
  try {
    driver.get(pathToFileURL(join(directory, 'diagram.svg')).href);
    const result = driver.execute_script(`
      const documentRoot = document.documentElement;
      if (document.querySelector('parsererror')) throw new Error('Invalid SVG');
      const result = {canvas: {width: documentRoot.getAttribute('width'), height: documentRoot.getAttribute('height')}, states: {}};
      for (const state of document.querySelectorAll('[data-state-id]')) {
        const nodes = [state, ...state.querySelectorAll('*')];
        const count = selector => nodes.filter(node => node.matches(selector)).length;
        const summary = nodes.map(node => node.firstChild?.nodeType === 3 ? node.firstChild.textContent : '').find(text => text.includes('spatial cards')) || '';
        result.states[state.getAttribute('data-state-id')] = {
          summary, local_paths: count('path[data-edge-kind]:not([data-edge-kind=""])'),
          index_cards: count('[data-relationship-index]:not([data-relationship-index=""])'),
          indexed_relationships: count('[data-relationship-entry]:not([data-relationship-entry=""])'),
          account_panels: count('[data-account-panel="true"]'), region_panels: count('[data-region]'), global_panels: count('[data-global="true"]')
        };
      }
      result.cross_paths = document.querySelectorAll('[data-cross-state="true"]').length;
      result.cross_references = JSON.parse(document.getElementById('cross-state-relationships').textContent).length;
      result.mappings = 3;
      return result;
    `);
    result.svg_bytes = statSync(join(directory, 'diagram.svg')).size;
    const html = join(directory, 'diagram.html');
    result.html_bytes = existsSync(html) ? statSync(html).size : null;
    for (const [name, state] of Object.entries(result.states)) {
      state.input = JSON.parse(readFileSync(join(directory, name + '.json'), 'utf8')).resource_changes.length;
    }
    console.log(JSON.stringify(result, null, 2));
  } finally { await driver.quit(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
