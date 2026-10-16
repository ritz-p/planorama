const { workerData } = require('node:worker_threads');
const { port, signal, url } = workerData;

port.on('message', async ({ method, path, body }) => {
  try {
    const response = await fetch(url + path, {
      method, headers: { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(30000),
    });
    const { value } = await response.json();
    if (!response.ok || value?.error) throw new Error(`${method} ${path}: ${value?.message || response.status}`);
    port.postMessage({ value });
  } catch (error) {
    port.postMessage({ error: error.stack });
  } finally {
    Atomics.store(signal, 0, 1);
    Atomics.notify(signal, 0);
  }
});
