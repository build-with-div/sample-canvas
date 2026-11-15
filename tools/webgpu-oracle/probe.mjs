// Which Chromium flags give WebGPU a SwiftShader adapter.
//
//     node tools/webgpu-oracle/probe.mjs CHROME
//
// Launches CHROME once with `--disable-gpu` (what the fidelity runners
// use) and once for every subset of CANDIDATES. Each launch asks for an
// adapter with default options and with `forceFallbackAdapter`, and checks
// that a WebGPU canvas's texture can be read back. Prints one JSON line
// for each launch. docs/webgpu-oracle.md records the results.

import { launch } from '../board-reference/chromium.mjs';
import { ready, serve } from './serve.mjs';

const CANDIDATES = [
  '--enable-unsafe-webgpu',
  '--use-webgpu-adapter=swiftshader',
  '--enable-features=Vulkan',
  '--use-vulkan=swiftshader',
  '--use-angle=vulkan',
];

const [chromePath] = process.argv.slice(2);
if (!chromePath) {
  console.error('usage: node tools/webgpu-oracle/probe.mjs CHROME');
  process.exit(2);
}

const sets = [['--disable-gpu']];
for (let mask = 0; mask < 1 << CANDIDATES.length; mask++) sets.push(CANDIDATES.filter((_, i) => mask & (1 << i)));

const server = await serve();
try {
  for (const gpu of sets) {
    const browser = await launch(chromePath, { width: 400, height: 300, gpu });
    try {
      await browser.cdp.load(server.url('webgpu-oracle/scenes.html'));
      await ready(browser.cdp, 'adapterInfo');
      const plain = await browser.cdp.evaluate('adapterInfo()');
      const fallback = await browser.cdp.evaluate('adapterInfo({ forceFallbackAdapter: true })');
      const canvas = plain.adapter ? await browser.cdp.evaluate('canvasReadback()') : null;
      console.log(JSON.stringify({ chromium: browser.version, platform: process.platform, flags: gpu, default: plain, forceFallbackAdapter: fallback, canvasReadback: canvas }));
    } finally {
      await browser.close();
    }
  }
} finally {
  await server.close();
}
