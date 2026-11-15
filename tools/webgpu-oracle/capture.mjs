// Renders the oracle scenes (scenes.js) in Chromium on SwiftShader WebGPU,
// RUNS times, each in a new browser, and reports whether the bytes repeat.
//
//     npm ci --prefix tools/three-oracle
//     node tools/webgpu-oracle/capture.mjs CHROME RUNS OUT.json [DUMP_DIR]
//
// CHROME is the `chrome` binary of Chrome for Testing 147.0.7727.50. Each
// run renders every scene twice in the same page. OUT.json records each
// render's SHA-256; DUMP_DIR, when given, receives the first run's bytes
// as <scene>.rgba for compare.mjs. docs/webgpu-oracle.md records the
// measurements.

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { launch } from '../board-reference/chromium.mjs';
import { ready, serve } from './serve.mjs';

// The fewest flags with which Chrome for Testing 147.0.7727.50 gives WebGPU
// a SwiftShader adapter whose canvas textures can be read back (probe.mjs,
// docs/webgpu-oracle.md). Windows picks the hardware GPU without
// `--use-webgpu-adapter`, and `--use-angle=vulkan` leaves it no adapter.
// Linux destroys the device once a canvas is configured unless the
// compositor runs on Vulkan.
const SWIFTSHADER = {
  win32: ['--enable-unsafe-webgpu', '--use-webgpu-adapter=swiftshader'],
  linux: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan'],
}[process.platform];
if (!SWIFTSHADER) {
  console.error(`no SwiftShader flags measured for ${process.platform}`);
  process.exit(2);
}

const [chromePath, runsArg, outPath, dumpDir] = process.argv.slice(2);
const runs = Number(runsArg);
if (!chromePath || !(runs >= 1) || !outPath) {
  console.error('usage: node tools/webgpu-oracle/capture.mjs CHROME RUNS OUT.json [DUMP_DIR]');
  process.exit(2);
}

const server = await serve();
const record = { chromium: null, platform: process.platform, flags: SWIFTSHADER, adapter: null, scenes: {} };
try {
  for (let run = 0; run < runs; run++) {
    const browser = await launch(chromePath, { width: 400, height: 300, gpu: SWIFTSHADER });
    try {
      record.chromium = browser.version;
      await browser.cdp.load(server.url('webgpu-oracle/scenes.html'));
      await ready(browser.cdp, 'renderScene');
      const names = await browser.cdp.evaluate('sceneNames()');
      for (const name of names) {
        for (let repeat = 0; repeat < 2; repeat++) {
          const bytes = Boolean(dumpDir) && run === 0 && repeat === 0;
          const result = await browser.cdp.evaluate(`renderScene(${JSON.stringify(name)}, { bytes: ${bytes} })`);
          if (!result.adapter.isFallbackAdapter || result.adapter.architecture !== 'swiftshader') {
            throw new Error(`${name} rendered on ${JSON.stringify(result.adapter)}, not SwiftShader`);
          }
          record.adapter = result.adapter;
          const scene = (record.scenes[name] ??= { width: result.width, height: result.height, format: result.format, renders: [] });
          scene.renders.push(result.sha256);
          if (bytes) {
            mkdirSync(dumpDir, { recursive: true });
            writeFileSync(join(dumpDir, `${name}.rgba`), Buffer.from(result.bytes, 'base64'));
          }
        }
      }
    } finally {
      await browser.close();
    }
    console.error(`run ${run + 1}/${runs} done`);
  }
} finally {
  await server.close();
}

for (const [name, scene] of Object.entries(record.scenes)) {
  scene.distinct = [...new Set(scene.renders)];
  console.log(`${name}: ${scene.renders.length} renders, ${scene.distinct.length} distinct (${scene.distinct.map((h) => h.slice(0, 12)).join(', ')})`);
}
writeFileSync(outPath, JSON.stringify(record, null, 1) + '\n');
