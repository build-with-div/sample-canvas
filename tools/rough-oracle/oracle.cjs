// Records what rough.js 4.6.6 itself produces for each case in
// samples/roughjs/tests/fixtures/cases.json (the generator) and
// canvas-cases.json (RoughCanvas's calls on its 2D context), as the port's
// oracle.
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

// RoughCanvas: record every access to the 2D context. A Proxy stands in for
// the context, so any member rough.js touches is recorded, including ones
// the port does not expect.
const METHODS = new Set(['save', 'restore', 'beginPath', 'moveTo', 'lineTo', 'bezierCurveTo', 'fill', 'stroke', 'setLineDash']);
function recordingContext(trace) {
  return new Proxy({}, {
    get(_, prop) {
      if (METHODS.has(prop)) {
        return (...args) => trace.push([prop, ...args.map((a) => (Array.isArray(a) ? [...a] : a))]);
      }
      trace.push(['get ' + String(prop)]);
      return undefined;
    },
    set(_, prop, value) {
      trace.push([String(prop) + '=', Array.isArray(value) ? [...value] : value]);
      return true;
    },
  });
}

const canvasCases = JSON.parse(fs.readFileSync(path.join(fixtures, 'canvas-cases.json'), 'utf8'));
const canvasResults = [];
for (const group of canvasCases.groups) {
  Math.random = mulberry32(group.mathRandomSeed);
  const trace = [];
  const contextKinds = [];
  const canvas = { getContext: (kind) => (contextKinds.push(kind), recordingContext(trace)) };
  const rc = rough.canvas(canvas, group.config || undefined);
  const calls = [];
  for (const call of group.calls) {
    const args = [...call.args];
    if (call.options) {
      args.push(call.options);
    }
    trace.length = 0;
    try {
      rc[call.method](...args);
      calls.push({ trace: [...trace] });
    } catch (error) {
      calls.push({ error: String(error && error.name), trace: [...trace] });
    }
  }
  canvasResults.push({ name: group.name, contextKinds, calls });
}
fs.writeFileSync(
  path.join(fixtures, 'rough-4.6.6-canvas.json'),
  JSON.stringify({ roughjs: out.roughjs, node: out.node, results: canvasResults }) + '\n',
);
console.log(`wrote ${canvasResults.length} canvas groups`);
