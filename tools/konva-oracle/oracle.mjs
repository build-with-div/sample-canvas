// Runs samples/konva/tests/fixtures/cases.json on Konva 10.9.1 and writes
// what Konva returned to samples/konva/tests/fixtures/konva-10.9.1.json.
//
//     npm ci --prefix tools/konva-oracle
//     node tools/konva-oracle/oracle.mjs
//
// Konva runs in Node without a DOM. Nothing is drawn: autoDraw is off and
// the canvases Konva creates for layers get a 2D context that ignores every
// call, so only the scene graph runs. See make_cases.py for the script
// format.

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const lib = (p) => pathToFileURL(join(here, 'node_modules', 'konva', 'lib', p)).href;

const { default: Konva } = await import(lib('Core.js'));
const { Rect } = await import(lib('shapes/Rect.js'));
const { Circle } = await import(lib('shapes/Circle.js'));

Konva.autoDrawEnabled = false;
Konva.showWarnings = false;
Konva.Util.createCanvasElement = () => ({
  style: {},
  width: 300,
  height: 300,
  getContext: () => new Proxy({}, { get: () => () => {}, set: () => true }),
});

const classes = { Stage: Konva.Stage, Layer: Konva.Layer, Group: Konva.Group, Rect, Circle };

// JSON has no NaN, Infinity or -0; they are written as strings.
function enc(v) {
  if (typeof v === 'number') {
    if (Number.isNaN(v)) return 'NaN';
    if (v === Infinity) return 'Infinity';
    if (v === -Infinity) return '-Infinity';
    if (Object.is(v, -0)) return '-0';
    return v;
  }
  if (Array.isArray(v)) return v.map(enc);
  if (v && typeof v === 'object') {
    const out = {};
    for (const [k, x] of Object.entries(v)) out[k] = enc(x);
    return out;
  }
  return v;
}

function runTransform(seq) {
  const t = new Konva.Transform();
  return seq.map(([op, ...args]) => {
    switch (op) {
      case 'translate':
      case 'scale':
      case 'skew':
        t[op](args[0], args[1]);
        return t.m.slice();
      case 'rotate':
        t.rotate(args[0]);
        return t.m.slice();
      case 'multiply':
        t.multiply(new Konva.Transform(args[0]));
        return t.m.slice();
      case 'invert':
        t.invert();
        return t.m.slice();
      case 'point':
        return t.point({ x: args[0], y: args[1] });
      case 'decompose':
        return t.decompose();
      case 'rect':
        return t._getTransformedRect({ x: args[0], y: args[1], width: args[2], height: args[3] });
      case 'isInvertible':
        return t.isInvertible();
      default:
        throw new Error(`unknown transform op ${op}`);
    }
  });
}

function runGroup(ops) {
  const nodes = new Map();
  const handles = new Map();
  const node = (h) => {
    const n = nodes.get(h);
    if (!n) throw new Error(`no node ${h}`);
    return n;
  };
  const handleOf = (n) => (n ? (handles.get(n) ?? '?') : null);
  let log = [];

  const query = (n, what, arg) => {
    switch (what) {
      case 'transform':
        return n.getTransform().m.slice();
      case 'absoluteTransform':
        return n.getAbsoluteTransform(arg ? node(arg) : undefined).m.slice();
      case 'absolutePosition':
        return n.getAbsolutePosition(arg ? node(arg) : undefined);
      case 'clientRect': {
        const config = { ...arg };
        if (config.relativeTo) config.relativeTo = node(config.relativeTo);
        return n.getClientRect(config);
      }
      case 'absoluteScale':
        return n.getAbsoluteScale();
      case 'absoluteRotation':
        return n.getAbsoluteRotation();
      case 'decompose':
        return n.getAbsoluteTransform().decompose();
      case 'absoluteOpacity':
        return n.getAbsoluteOpacity();
      case 'isVisible':
        return n.isVisible();
      case 'isListening':
        return n.isListening();
      case 'zIndex':
        return n.getZIndex();
      case 'absoluteZIndex':
        return n.getAbsoluteZIndex();
      case 'depth':
        return n.getDepth();
      case 'attrs':
        return Object.entries(n.getAttrs());
      case 'children':
        return n.getChildren().map(handleOf);
      case 'parent':
        return handleOf(n.getParent());
      case 'layer':
        return handleOf(n.getLayer());
      case 'stage':
        return handleOf(n.getStage());
      case 'position':
        return n.getPosition();
      case 'size':
        return n.getSize();
      default:
        throw new Error(`unknown query ${what}`);
    }
  };

  return ops.map((op) => {
    log = [];
    let r = null;
    const [kind, h] = op;
    switch (kind) {
      case 'new': {
        const n = new classes[op[2]](Object.fromEntries(op[3]));
        nodes.set(h, n);
        handles.set(n, h);
        break;
      }
      case 'add':
        node(h).add(...op[2].map(node));
        break;
      case 'set':
        node(h).setAttrs(Object.fromEntries(op[2]));
        break;
      case 'call': {
        const [, , method, arg] = op;
        const n = node(h);
        let result;
        if (method === 'moveTo') result = n.moveTo(node(arg));
        else if (arg === null || arg === undefined) result = n[method]();
        else result = n[method](arg);
        r = typeof result === 'boolean' ? result : null;
        break;
      }
      case 'on': {
        const [, , events, label, cancelBubble] = op;
        node(h).on(events, (evt) => {
          const entry = {
            l: label,
            type: evt.type,
            target: handleOf(evt.target),
            current: handleOf(evt.currentTarget),
          };
          if ('oldVal' in evt) entry.old = evt.oldVal ?? null;
          if ('newVal' in evt) entry.new = evt.newVal ?? null;
          if (evt.child) entry.child = handleOf(evt.child);
          log.push(entry);
          if (cancelBubble) evt.cancelBubble = true;
        });
        break;
      }
      case 'off':
        node(h).off(op[2] ?? undefined);
        break;
      case 'fire':
        node(h).fire(op[2], {}, op[3]);
        break;
      case 'get':
        r = query(node(h), op[2], op[3]);
        break;
      case 'tf':
        r = runTransform(op[1]);
        break;
      default:
        throw new Error(`unknown op ${kind}`);
    }
    const out = { r: enc(r) };
    if (log.length) out.events = enc(log);
    return out;
  });
}

const cases = JSON.parse(readFileSync(join(root, 'samples/konva/tests/fixtures/cases.json'), 'utf8'));
const groups = cases.groups.map((g) => ({ name: g.name, results: runGroup(g.ops) }));
// Math.hypot on seeded pairs (mulberry32, as samples/roughjs/tests/common),
// for js::hypot: Konva sizes strokes with it when strokeScaleEnabled is off.
function mulberry32(seed) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const rand = mulberry32(1091);
const hypot = [];
for (let i = 0; i < 4000; i++) {
  const scale = 10 ** Math.floor(rand() * 12 - 6);
  const a = (rand() * 2 - 1) * scale;
  const b = i % 4 === 0 ? (rand() * 2 - 1) * scale * 1e-9 : (rand() * 2 - 1) * scale;
  hypot.push([a, b, Math.hypot(a, b)]);
}
const out = { konva: Konva.version, node: process.version, groups, hypot };
writeFileSync(join(root, 'samples/konva/tests/fixtures/konva-10.9.1.json'), JSON.stringify(out) + '\n');
const count = groups.reduce((n, g) => n + g.results.length, 0);
console.log(`Konva ${Konva.version}: ${groups.length} groups, ${count} results`);
