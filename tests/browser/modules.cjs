const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const test = require('node:test');
const context = vm.createContext({});
for (const name of ['lifecycle', 'highlighting']) {
  vm.runInContext(fs.readFileSync(`src/html/${name}.js`, 'utf8'), context);
}

test('lifecycle resets each registered feature on every transition', () => {
  const lifecycle = context.lifecycle();
  const calls = [];
  lifecycle.register({ reset: () => calls.push('diagnostics') });
  lifecycle.register({ reset: () => calls.push('selection') });
  lifecycle.reset();
  lifecycle.reset();
  assert.deepEqual(calls, ['diagnostics', 'selection', 'diagnostics', 'selection']);
});

test('highlight reset removes all selection roles without touching edge styling', () => {
  const attrs = new Map([['stroke', 'red'], ['data-viewer-edge', 'provenance'], ['data-viewer-related', 'true']]);
  const element = { removeAttribute: key => attrs.delete(key) };
  const viewport = new Map([['data-selection', 'true']]);
  context.highlighting({ querySelectorAll: () => [element] }, { removeAttribute: key => viewport.delete(key) }).clear();
  assert.deepEqual([...attrs], [['stroke', 'red']]);
  assert.equal(viewport.size, 0);
});
