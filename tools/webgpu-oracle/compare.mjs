// Compares two DUMP_DIRs written by capture.mjs, scene by scene.
//
//     node tools/webgpu-oracle/compare.mjs DIR_A DIR_B
//
// Prints, for each <scene>.rgba in both, the number of pixels whose bytes
// differ and the largest difference in one channel.

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const [a, b] = process.argv.slice(2);
if (!a || !b) {
  console.error('usage: node tools/webgpu-oracle/compare.mjs DIR_A DIR_B');
  process.exit(2);
}

for (const file of readdirSync(a).filter((f) => f.endsWith('.rgba')).sort()) {
  const x = readFileSync(join(a, file));
  let y;
  try {
    y = readFileSync(join(b, file));
  } catch {
    console.log(`${file}: only in ${a}`);
    continue;
  }
  if (x.length !== y.length) {
    console.log(`${file}: ${x.length} bytes vs ${y.length}`);
    continue;
  }
  let pixels = 0;
  let max = 0;
  for (let i = 0; i < x.length; i += 4) {
    let differs = false;
    for (let c = 0; c < 4; c++) {
      const d = Math.abs(x[i + c] - y[i + c]);
      if (d) differs = true;
      if (d > max) max = d;
    }
    if (differs) pixels++;
  }
  console.log(`${file}: ${pixels} of ${x.length / 4} pixels differ, max channel difference ${max}`);
}
