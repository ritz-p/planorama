const element = id => document.getElementById(id);
let assets;
let worker;
let resultUrl;
let previewUrl;
let serial = 0;
let elapsedTimer;
const stages = Object.freeze({parsing: 'Parsing plan', selection: 'Interpreting and selecting resources', placement: 'Placing resources', routing: 'Routing connections', output: 'Composing output'});

function startElapsed() {
  const started = performance.now();
  const update = () => { element('elapsed').textContent = `Elapsed: ${Math.floor((performance.now() - started) / 1000)} s`; };
  element('elapsed').hidden = false;
  update();
  elapsedTimer = setInterval(update, 1000);
}
const maxBytes = 32 * 1024 * 1024;
let moduleId = 0;
let inventory = [];
let resources = [];
let resourcePage = 0;
let inventoryPage = 0;
let inspecting = false;
const pageSize = 50;

function renderResources() {
  const focus = element('focus').value;
  const query = element('resource-search').value.toLowerCase();
  const matches = resources.filter(entry => [entry.address, entry.resource_type, entry.module, entry.mode, entry.state ?? 'single plan'].some(value => value.toLowerCase().includes(query)));
  resourcePage = Math.max(0, Math.min(resourcePage, Math.ceil(matches.length / pageSize) - 1));
  const start = resourcePage * pageSize;
  element('resource-selection').textContent = focus === '' ? 'No focus selected.' : `Focus: ${focus}. ${resources.some(entry => entry.selector === focus) ? 'Present in original input; generation validates selection.' : 'Not in the completed list; generation validates this manual selector.'}`;
  element('resource-count').textContent = `${matches.length} of ${resources.length} addresses match. Showing ${matches.length ? start + 1 : 0}–${Math.min(start + pageSize, matches.length)}.`;
  element('resource-previous').disabled = resourcePage === 0;
  element('resource-next').disabled = start + pageSize >= matches.length;
  element('resource-inventory').replaceChildren(...matches.slice(start, start + pageSize).map(entry => {
    const label = document.createElement('label');
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = entry.selector === focus;
    checkbox.value = entry.selector;
    checkbox.onchange = () => {
      element('focus').value = checkbox.checked ? entry.selector : '';
      settingsChanged();
      Array.from(element('resource-inventory').querySelectorAll('input')).find(input => input.value === entry.selector)?.focus();
    };
    label.append(checkbox, ` ${entry.address} — ${entry.resource_type}; ${entry.module}; ${entry.mode}; state: ${entry.state ?? 'single plan (unnamed)'}; current ${entry.current}; deposed ${entry.deposed}`);
    return label;
  }));
}

function renderInventory() {
  const selected = new Set(Array.from(element('modules').querySelectorAll('input'), input => input.value).filter(value => value !== ''));
  const query = element('module-search').value.toLowerCase();
  const matches = inventory.filter(entry => (entry.selector ?? '(root)').toLowerCase().includes(query));
  inventoryPage = Math.max(0, Math.min(inventoryPage, Math.ceil(matches.length / pageSize) - 1));
  const start = inventoryPage * pageSize;
  element('module-count').textContent = `${selected.size} selectors selected. ${matches.length} of ${inventory.length} rows match. Showing ${matches.length ? start + 1 : 0}–${Math.min(start + pageSize, matches.length)}.`;
  element('module-previous').disabled = inventoryPage === 0;
  element('module-next').disabled = start + pageSize >= matches.length;
  element('module-inventory').replaceChildren(...matches.slice(start, start + pageSize).map(entry => {
    const label = document.createElement('label');
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.disabled = entry.selector === null;
    checkbox.checked = selected.has(entry.selector);
    checkbox.value = entry.selector ?? '';
    checkbox.onchange = () => {
      if (checkbox.checked) addModule(entry.selector, false);
      else {
        for (const input of element('modules').querySelectorAll('input')) {
          if (input.value === entry.selector) input.closest('.module-row').remove();
        }
        settingsChanged();
      }
      Array.from(element('module-inventory').querySelectorAll('input')).find(input => input.value === entry.selector)?.focus();
    };
    label.append(checkbox, ` ${entry.selector ?? '(root; not selectable)'} — ${entry.instance ? 'instance' : 'configuration (aggregate)'}; direct ${entry.direct}; subtree ${entry.subtree}`);
    return label;
  }));
}

function settingsChanged() {
  stop();
  clearResult();
  element('focus-depth').disabled = element('focus').value === '';
  element('settings-error').textContent = '';
  document.querySelectorAll('[aria-invalid]').forEach(input => {
    input.removeAttribute('aria-invalid');
    input.setAttribute('aria-describedby', settingHelp(input));
  });
  if (assets) element('status').textContent = 'Settings changed. Generate again to apply them.';
  renderInventory();
  renderResources();
}

element('generate-form').addEventListener('input', event => {
  if (!['module-search', 'resource-search'].includes(event.target.id) && !element('module-inventory').contains(event.target) && !element('resource-inventory').contains(event.target)) settingsChanged();
});
element('file').addEventListener('change', () => {
  inventory = [];
  resources = [];
  resourcePage = 0;
  element('resource-search').value = '';
  element('focus').value = '';
  inventoryPage = 0;
  element('module-search').value = '';
  element('modules').replaceChildren();
  settingsChanged();
  inspectModules();
});
element('module-search').oninput = () => { inventoryPage = 0; renderInventory(); };
element('module-search').onkeydown = event => {
  if (event.key === 'Enter') event.preventDefault();
};
element('module-previous').onclick = () => { inventoryPage--; renderInventory(); };
element('module-next').onclick = () => { inventoryPage++; renderInventory(); };
element('inspect-modules').onclick = inspectModules;
element('inspect-resources').onclick = inspectModules;
element('resource-search').oninput = () => { resourcePage = 0; renderResources(); };
element('resource-search').onkeydown = event => { if (event.key === 'Enter') event.preventDefault(); };
element('resource-previous').onclick = () => { resourcePage--; renderResources(); };
element('resource-next').onclick = () => { resourcePage++; renderResources(); };
element('clear-focus').onclick = () => { element('focus').value = ''; settingsChanged(); };
element('add-module').onclick = () => addModule();
function addModule(value = '', focus = true) {
  const row = document.createElement('div');
  row.className = 'module-row';
  const label = document.createElement('label');
  const input = document.createElement('input');
  input.type = 'text';
  input.value = value;
  input.id = `module-${++moduleId}`;
  input.autocomplete = 'off';
  input.spellcheck = false;
  input.setAttribute('aria-describedby', 'modules-help');
  label.htmlFor = input.id;
  label.append('Module selector ', input);
  const remove = document.createElement('button');
  remove.type = 'button';
  remove.textContent = 'Remove module';
  remove.onclick = () => {
    const next = row.nextElementSibling?.querySelector('input') || row.previousElementSibling?.querySelector('input') || element('add-module');
    row.remove();
    settingsChanged();
    next.focus();
  };
  row.append(label, remove);
  element('modules').append(row);
  settingsChanged();
  if (focus) input.focus();
}

element('reset-settings').onclick = () => {
  element('format').value = 'svg';
  element('changed').checked = false;
  element('address-format').value = 'qualified';
  element('icon-mode').value = 'icon';
  element('hide-legend').checked = false;
  element('html-diagnostics').checked = false;
  element('html-semantic-diagnostics').checked = false;
  element('focus').value = '';
  element('focus-depth').value = '';
  element('modules').replaceChildren();
  element('module-search').value = '';
  element('resource-search').value = '';
  resourcePage = 0;
  inventoryPage = 0;
  element('spatial-types').querySelectorAll('input').forEach(input => { input.checked = false; });
  settingsChanged();
  if (assets) element('status').textContent = 'Settings reset to defaults. Generate again; the selected file is retained.';
};

function settingHelp(input) {
  return input.id.startsWith('html-') ? 'diagnostics-help' : input.id === 'focus-depth' ? 'depth-help' : 'modules-help';
}

function invalidSetting(input, message) {
  input.setAttribute('aria-invalid', 'true');
  input.setAttribute('aria-describedby', settingHelp(input) + ' settings-error');
  element('settings-error').textContent = message;
  input.focus();
}

function snapshotOptions() {
  const options = {
    format: element('format').value,
    changed_only: element('changed').checked,
    address_format: element('address-format').value,
    icon_mode: element('icon-mode').value,
    hide_relationship_legend: element('hide-legend').checked,
    html_diagnostics: element('html-diagnostics').checked,
    html_semantic_diagnostics: element('html-semantic-diagnostics').checked,
    modules: [],
    spatial_types: Array.from(element('spatial-types').querySelectorAll('input:checked'), input => input.value),
  };
  if (options.format !== 'html' && (options.html_diagnostics || options.html_semantic_diagnostics)) {
    invalidSetting(element(options.html_semantic_diagnostics ? 'html-semantic-diagnostics' : 'html-diagnostics'), 'HTML diagnostics require HTML output. Choose HTML or turn off both diagnostic options.');
    return;
  }
  if (element('focus').value !== '') {
    options.focus = element('focus').value;
    const depth = element('focus-depth');
    if (depth.value !== '') {
      if (!/^[0-9]+$/.test(depth.value) || Number(depth.value) > 4294967295) {
        invalidSetting(depth, 'Focus depth must be a nonnegative integer from 0 to 4294967295, or empty for the default.');
        return;
      }
      options.focus_depth = Number(depth.value);
    }
  }
  for (const input of element('modules').querySelectorAll('input')) {
    if (input.value === '') {
      invalidSetting(input, 'Enter a module selector or remove the empty row.');
      return;
    }
    options.modules.push(input.value);
  }
  return options;
}

function clearResult() {
  element('download').hidden = true;
  element('download').removeAttribute('href');
  element('preview').removeAttribute('src');
  if (resultUrl) URL.revokeObjectURL(resultUrl);
  if (previewUrl) URL.revokeObjectURL(previewUrl);
  resultUrl = previewUrl = undefined;
}

function stop() {
  clearInterval(elapsedTimer);
  elapsedTimer = undefined;
  element('elapsed').hidden = true;
  serial++;
  if (worker) worker.terminate();
  worker = undefined;
  if (inspecting) element('inventory-status').textContent = 'Module and resource listing cancelled. Reload either list to retry.';
  inspecting = false;
  element('generate').disabled = !assets;
  element('cancel').disabled = true;
}

async function loadAssets() {
  const load = async path => {
    const response = await fetch(path, {credentials: 'omit'});
    if (!response.ok) throw new Error('asset');
    return response;
  };
  const [glue, code, bytes] = await Promise.all([
    load('pkg/planorama.js').then(response => response.text()),
    load('worker.js').then(response => response.text()),
    load('pkg/planorama_bg.wasm').then(response => response.arrayBuffer()),
  ]);
  assets = {module: await WebAssembly.compile(bytes), url: URL.createObjectURL(new Blob([glue, '\n', code], {type: 'text/javascript'}))};
  element('generate').disabled = false;
  element('status').textContent = 'Ready. Generation works offline while this page remains open.';
  element('inspect-modules').disabled = false;
  element('inspect-resources').disabled = false;
  if (element('file').files[0]) inspectModules();
}

async function inspectModules() {
  stop();
  clearResult();
  const request = serial;
  const file = element('file').files[0];
  if (!file || !assets) {
    element('inventory-status').textContent = 'Select a file after the generator is ready.';
    return;
  }
  if (file.size > maxBytes) {
    element('inventory-status').textContent = 'Input exceeds the 32 MiB limit.';
    return;
  }
  inspecting = true;
  element('cancel').disabled = false;
  element('inventory-status').textContent = 'Listing modules and resources…';
  try {
    const json = await file.text();
    if (request !== serial) return;
    worker = new Worker(assets.url);
    worker.onerror = event => {
      event.preventDefault();
      if (request !== serial) return;
      stop();
      element('inventory-status').textContent = 'Module and resource listing failed. Try a smaller plan.';
    };
    worker.onmessage = ({data}) => {
      if (request !== serial) return;
      inspecting = false;
      stop();
      inventory = data.ok ? data.modules : [];
      resources = data.ok ? data.resources : [];
      renderInventory();
      renderResources();
      element('inventory-status').textContent = data.ok ? 'Modules from the original input, with resource addresses listed above. No selection filters applied.' : data.error.message;
    };
    worker.postMessage({module: assets.module, json, operation: 'inspect'});
  } catch {
    if (request !== serial) return;
    stop();
    element('inventory-status').textContent = 'Could not read the selected file or list modules and resources.';
  }
}

element('cancel').onclick = () => {
  stop();
  clearResult();
  element('status').textContent = 'Cancelled. Select a file or generate again.';
};

element('generate-form').onsubmit = async event => {
  event.preventDefault();
  stop();
  clearResult();
  const request = serial;
  const file = element('file').files[0];
  if (!file || !assets) return;
  if (file.size > maxBytes) {
    element('status').textContent = 'Input exceeds the 32 MiB limit.';
    return;
  }
  const options = snapshotOptions();
  if (!options) return;
  const {format} = options;
  element('generate').disabled = true;
  element('cancel').disabled = false;
  element('status').textContent = 'Generating…';
  startElapsed();
  try {
    const json = await file.text();
    if (request !== serial) return;
    worker = new Worker(assets.url);
    const activeWorker = worker;
    worker.onerror = event => {
      event.preventDefault();
      if (request !== serial) return;
      stop();
      element('status').textContent = 'Diagram generation failed. Try a smaller plan.';
    };
    worker.onmessage = ({data}) => {
      if (request !== serial) return;
      if (worker !== activeWorker) return;
      if (data.type === 'progress') {
        if (data.request === request && Object.hasOwn(stages, data.stage)) {
          const message = `${stages[data.stage]}…`;
          if (element('status').textContent !== message) element('status').textContent = message;
        }
        return;
      }
      stop();
      if (!data.ok) {
        element('status').textContent = data.error.message;
        return;
      }
      const type = format === 'svg' ? 'image/svg+xml' : 'text/html';
      resultUrl = URL.createObjectURL(new Blob([data.content], {type}));
      const policy = '<meta http-equiv="Content-Security-Policy" content="default-src &apos;none&apos;; script-src &apos;unsafe-inline&apos;; style-src &apos;unsafe-inline&apos;; img-src data:; connect-src &apos;none&apos;; frame-src &apos;none&apos;; form-action &apos;none&apos;; base-uri &apos;none&apos;">';
      const preview = format === 'html' ? data.content.replace('<head>', '<head>' + policy) : '<!doctype html><head>' + policy + '</head><body>' + data.content;
      previewUrl = URL.createObjectURL(new Blob([preview], {type: 'text/html'}));
      element('preview').src = previewUrl;
      element('download').href = resultUrl;
      element('download').download = 'diagram.' + format;
      element('download').textContent = 'Download ' + format.toUpperCase();
      element('download').hidden = false;
      element('status').textContent = `Generated in ${Math.round(data.elapsed_ms)} ms. ${data.diagnostics.join(' ')}`;
    };
    worker.postMessage({module: assets.module, json, options, request});
  } catch {
    if (request !== serial) return;
    stop();
    element('status').textContent = 'Could not read the selected file or start the generator.';
  }
};

loadAssets().catch(() => {
  element('status').textContent = 'Could not load the generator. Reload while connected.';
});
