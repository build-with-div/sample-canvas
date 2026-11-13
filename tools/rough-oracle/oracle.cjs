// Records what rough.js 4.6.6 itself produces for each case in
// samples/roughjs/tests/fixtures/cases.json, as the port's oracle.
//
//   cd tools/rough-oracle && npm ci && node oracle.cjs
//
// It runs the package's own bundle (`roughjs/bundled/rough.cjs.js`, the
// build its package.json points at). Math.random is replaced with
// mulberry32 from a fixed seed, which samples/roughjs/tests/oracle.rs
// reproduces, so that unseeded shapes and the `dots` fill are comparable.
'use strict';

const fs = require('fs');
const path = require('path');
const rough = require('roughjs/bundled/rough.cjs.js');

const fixtures = path.join(__dirname, '..', '..', 'samples', 'roughjs', 'tests', 'fixtures');
const cases = JSON.parse(fs.readFileSync(path.join(fixtures, 'cases.json'), 'utf8'));

function mulberry32(a) {
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function drawableJson(generator, drawable) {
  return {
    shape: drawable.shape,
    sets: drawable.sets.map((set) => ({
      type: set.type,
      ops: set.ops.map((op) => ({ op: op.op, data: op.data })),
    })),
    paths: generator.toPaths(drawable),
    fixed2: drawable.sets.map((set) => generator.opsToPath(set, 2)),
  };
}

const results = [];
for (const group of cases.groups) {
  Math.random = mulberry32(group.mathRandomSeed);
  const generator = rough.generator(group.config || undefined);
  const calls = [];
  for (const call of group.calls) {
    const args = [...call.args];
    if (call.options) {
      args.push(call.options);
    }
    try {
      calls.push(drawableJson(generator, generator[call.method](...args)));
    } catch (error) {
      calls.push({ error: String(error && error.name) });
    }
  }
  results.push({ name: group.name, calls });
}

const out = {
  roughjs: require('roughjs/package.json').version,
  node: process.version,
  results,
};
fs.writeFileSync(path.join(fixtures, 'rough-4.6.6.json'), JSON.stringify(out) + '\n');
console.log(`wrote ${results.length} groups, ${results.reduce((n, g) => n + g.calls.length, 0)} calls`);
