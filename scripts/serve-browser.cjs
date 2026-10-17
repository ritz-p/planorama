const http = require('node:http');
const {readFile} = require('node:fs/promises');
const {resolve} = require('node:path');
const root = resolve(__dirname, '../output/browser');
const files = new Map([
  ['/', ['index.html', 'text/html']],
  ['/index.html', ['index.html', 'text/html']],
  ['/app.js', ['app.js', 'text/javascript']],
  ['/worker.js', ['worker.js', 'text/javascript']],
  ['/pkg/planorama.js', ['pkg/planorama.js', 'text/javascript']],
  ['/pkg/planorama_bg.wasm', ['pkg/planorama_bg.wasm', 'application/wasm']],
]);
const server = http.createServer(async (request, response) => {
  const file = files.get(request.url);
  if (request.method !== 'GET' || !file) {
    response.writeHead(404).end();
    return;
  }
  try {
    const content = await readFile(resolve(root, file[0]));
    response.writeHead(200, {'Content-Type': file[1], 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff'}).end(content);
  } catch {
    response.writeHead(500).end('Build the browser assets first.');
  }
});
server.listen(Number(process.env.PORT || 8080), process.env.HOST || '127.0.0.1', () => {
  if (process.send) process.send({port: server.address().port});
  else console.log(`Planorama: http://localhost:${server.address().port}`);
});
