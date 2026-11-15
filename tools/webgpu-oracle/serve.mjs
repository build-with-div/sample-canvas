// Serves tools/ on 127.0.0.1 for the oracle pages, which load three.js from
// tools/three-oracle/node_modules. Pages must come from http: WebGPU needs a
// secure context and module scripts don't load from file:.

import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const tools = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json' };

// Resolves to { url(path), close() }.
export async function serve() {
  const server = createServer((request, response) => {
    const path = decodeURIComponent(new URL(request.url, 'http://127.0.0.1').pathname);
    const file = normalize(join(tools, path));
    if (!file.startsWith(tools + sep)) {
      response.writeHead(403).end();
      return;
    }
    try {
      const body = readFileSync(file);
      response.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' }).end(body);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise((done) => server.listen(0, '127.0.0.1', done));
  const { port } = server.address();
  return {
    url: (path) => `http://127.0.0.1:${port}/${path}`,
    close: () => new Promise((done) => server.close(done)),
  };
}

// Waits until `window[name]` is a function in the loaded page.
export async function ready(cdp, name) {
  for (let attempt = 0; attempt < 200; attempt++) {
    if (await cdp.evaluate(`typeof window.${name} === "function"`)) return;
    await new Promise((done) => setTimeout(done, 50));
  }
  throw new Error(`the page never defined ${name}`);
}
