// Replays samples/three-shell/tests/fixtures/fidelity/shell-scenarios.json
// on the three.js shell reference page (shell.html) in pinned Chromium, over
// the DevTools protocol, and writes what the page shows after each step.
//
//     npm ci --prefix tools/three-oracle
//     node tools/three-reference/capture.mjs CHROME samples/three-shell/tests/fixtures/fidelity/shell-chromium.json
//
// CHROME is the `chrome` binary of Chrome for Testing 147.0.7727.50, the
// build Div's census pins. samples/three-shell/tests/fidelity_shell.rs
// replays the same steps on Div and compares.
//
// Input arrives as trusted browser input (Input.dispatchMouseEvent and
// Input.dispatchKeyEvent), so Chromium produces the pointer, mouse, wheel,
// click and key events itself. After each step the page settles (two
// frames and a task, so the ResizeObserver has run), then runs one
// animation frame of the app (`controls.update()`), then is read. The page
// is served from tools/ on 127.0.0.1, since its modules can't load from
// file: URLs.

import { createServer } from 'node:http';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { launch } from '../board-reference/chromium.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const tools = resolve(here, '..');
const root = resolve(tools, '..');
const [chromePath, outputPath] = process.argv.slice(2);
if (!chromePath || !outputPath) {
  console.error('usage: node tools/three-reference/capture.mjs CHROME OUTPUT');
  process.exit(2);
}
const spec = JSON.parse(readFileSync(join(root, 'samples/three-shell/tests/fixtures/fidelity/shell-scenarios.json'), 'utf8'));
const three = JSON.parse(readFileSync(join(tools, 'three-oracle/node_modules/three/package.json'), 'utf8')).version;

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json' };
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
const page = `http://127.0.0.1:${server.address().port}/three-reference/shell.html`;

// key: [code, Windows virtual-key code]
const KEYS = {
  ArrowLeft: ['ArrowLeft', 37],
  ArrowUp: ['ArrowUp', 38],
  ArrowRight: ['ArrowRight', 39],
  ArrowDown: ['ArrowDown', 40],
  Escape: ['Escape', 27],
};
// DevTools modifier bits.
const MODIFIERS = { alt: 1, ctrl: 2, meta: 4, shift: 8 };
// MouseEvent.buttons bits.
const BUTTONS = { left: 1, right: 2, middle: 4 };

function modifiers(entry) {
  return (entry.modifiers ?? []).reduce((bits, name) => bits | MODIFIERS[name], 0);
}

async function mouse(cdp, kind, x, y, button, bits, pressed) {
  const type = { move: 'mouseMoved', down: 'mousePressed', up: 'mouseReleased' }[kind];
  if (kind === 'down') pressed.buttons |= BUTTONS[button];
  if (kind === 'up') pressed.buttons &= ~BUTTONS[button];
  await cdp.send('Input.dispatchMouseEvent', {
    type,
    x,
    y,
    button: kind === 'move' ? 'none' : button,
    buttons: pressed.buttons,
    clickCount: kind === 'move' ? 0 : 1,
    modifiers: bits,
    pointerType: 'mouse',
  });
}

async function step(cdp, entry, pressed) {
  if (entry.mouse) {
    await mouse(cdp, entry.mouse, entry.x, entry.y, entry.button ?? 'left', modifiers(entry), pressed);
  } else if (entry.click) {
    const [x, y] = entry.click;
    for (const kind of ['move', 'down', 'up']) await mouse(cdp, kind, x, y, 'left', 0, pressed);
  } else if (entry.wheel) {
    const [x, y, deltaY] = entry.wheel;
    await cdp.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: 0, deltaY, modifiers: modifiers(entry) });
  } else if (entry.key) {
    const [code, vk] = KEYS[entry.key] ?? [];
    if (!code) throw new Error(`unknown key ${entry.key}`);
    const key = { key: entry.key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk, modifiers: modifiers(entry) };
    await cdp.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
    await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
  } else if (!entry.idle) {
    throw new Error(`unknown step ${JSON.stringify(entry)}`);
  }
}

const browser = await launch(chromePath, spec.host);
const scenarios = {};
try {
  const { cdp } = browser;
  for (const scenario of spec.scenarios) {
    await cdp.load(`${page}?${scenario.id}`);
    for (let attempt = 0; attempt < 200; attempt++) {
      if (await cdp.evaluate('window.__shell?.ready === true')) break;
      await new Promise((done) => setTimeout(done, 50));
    }
    await cdp.evaluate('window.__shell.settle()');
    const snapshots = [await cdp.evaluate('window.__shell.snapshot()')];
    const pressed = { buttons: 0 };
    for (const entry of scenario.steps) {
      await step(cdp, entry, pressed);
      await cdp.evaluate('window.__shell.settle()');
      await cdp.evaluate('window.__shell.frame()');
      snapshots.push(await cdp.evaluate('window.__shell.snapshot()'));
    }
    scenarios[scenario.id] = snapshots;
    console.log(`${scenario.id}: ${snapshots.length} snapshots`);
  }
} finally {
  await browser.close();
  server.close();
}

// One snapshot per line keeps the fixture reviewable.
const comment =
  'Generated by tools/three-reference/capture.mjs from shell-scenarios.json; do not edit. ' +
  'Snapshot 0 is after load, snapshot n after step n and one frame.';
const lines = Object.entries(scenarios).map(
  ([id, snapshots]) => `    ${JSON.stringify(id)}: [\n${snapshots.map((s) => `      ${JSON.stringify(s)}`).join(',\n')}\n    ]`,
);
writeFileSync(
  outputPath,
  [
    '{',
    `  "comment": ${JSON.stringify(comment)},`,
    `  "chromium": ${JSON.stringify(browser.version)},`,
    `  "three": ${JSON.stringify(three)},`,
    '  "scenarios": {',
    lines.join(',\n'),
    '  }',
    '}',
    '',
  ].join('\n'),
);
console.log(browser.version);
