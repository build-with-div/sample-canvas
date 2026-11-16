// Records the WGSL that three.js 0.187.0's node builder generates for the
// scene's MeshBasicMaterial, into samples/three-render/src/wgsl. The port
// draws with these shaders; it does not port the node system (TSL) that
// writes them.
//
//     npm ci --prefix tools/three-oracle
//     node tools/three-render-reference/record_wgsl.mjs CHROME
//
// Renders the first view of scene.json with `GPUDevice.createShaderModule`
// wrapped, and writes each module under its label (`vertex`, `fragment`).
// Fails if three.js creates other modules, or differing ones under one
// label.

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch } from '../board-reference/chromium.mjs';
import { swiftshaderFlags } from '../webgpu-oracle/flags.mjs';
import { ready, serve } from '../webgpu-oracle/serve.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const render = join(root, 'samples', 'three-render');
const [chromePath] = process.argv.slice(2);
if (!chromePath) {
  console.error('usage: node tools/three-render-reference/record_wgsl.mjs CHROME');
  process.exit(2);
}
const spec = JSON.parse(readFileSync(join(render, 'tests', 'fixtures', 'scene.json'), 'utf8'));
spec.views = spec.views.slice(0, 1);

const WRAP = `(() => {
  const modules = (window.shaderModules = []);
  const create = GPUDevice.prototype.createShaderModule;
  GPUDevice.prototype.createShaderModule = function (descriptor) {
    modules.push({ label: descriptor.label, code: descriptor.code });
    return create.call(this, descriptor);
  };
})()`;

const server = await serve();
const browser = await launch(chromePath, { width: 400, height: 300, gpu: swiftshaderFlags() });
let modules;
try {
  await browser.cdp.load(server.url('three-render-reference/scene.html'));
  await ready(browser.cdp, 'renderScene');
  await browser.cdp.evaluate(WRAP);
  await browser.cdp.evaluate(`renderScene(${JSON.stringify(spec)})`);
  modules = await browser.cdp.evaluate('window.shaderModules');
} finally {
  await browser.close();
  await server.close();
}

const FILES = { vertex: 'mesh-basic.vertex.wgsl', fragment: 'mesh-basic.fragment.wgsl' };
const byLabel = {};
for (const { label, code } of modules) {
  if (!(label in FILES)) throw new Error(`unexpected shader module ${JSON.stringify(label)}`);
  if (label in byLabel && byLabel[label] !== code) throw new Error(`two different ${label} shaders`);
  byLabel[label] = code;
}
for (const [label, file] of Object.entries(FILES)) {
  if (!(label in byLabel)) throw new Error(`no ${label} shader`);
  writeFileSync(join(render, 'src', 'wgsl', file), byLabel[label]);
}
console.log(`${modules.length} shader modules, ${Object.keys(byLabel).length} distinct, ${browser.version}`);
