const element = id => document.getElementById(id);
let assets;
let worker;
let resultUrl;
let previewUrl;
let serial = 0;
const maxBytes = 32 * 1024 * 1024;

function clearResult() {
  element('download').hidden = true;
  element('download').removeAttribute('href');
  element('preview').removeAttribute('src');
  if (resultUrl) URL.revokeObjectURL(resultUrl);
  if (previewUrl) URL.revokeObjectURL(previewUrl);
  resultUrl = previewUrl = undefined;
}

function stop() {
  serial++;
  if (worker) worker.terminate();
  worker = undefined;
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
  const format = element('format').value;
  const options = {format, changed_only: element('changed').checked};
  element('generate').disabled = true;
  element('cancel').disabled = false;
  element('status').textContent = 'Generating…';
  try {
    const json = await file.text();
    if (request !== serial) return;
    worker = new Worker(assets.url);
    worker.onerror = event => {
      event.preventDefault();
      if (request !== serial) return;
      stop();
      element('status').textContent = 'Diagram generation failed. Try a smaller plan.';
    };
    worker.onmessage = ({data}) => {
      if (request !== serial) return;
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
    worker.postMessage({module: assets.module, json, options});
  } catch {
    if (request !== serial) return;
    stop();
    element('status').textContent = 'Could not read the selected file or start the generator.';
  }
};

loadAssets().catch(() => {
  element('status').textContent = 'Could not load the generator. Reload while connected.';
});
