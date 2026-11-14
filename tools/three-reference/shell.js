// The three.js example shell, as a web page: the reference the Div shell in
// samples/three-shell is compared with. It builds the same DOM with the same
// inline styles and runs the same app code, on three.js 0.187.0 itself:
// a toolbar, an objects pane, a splitter and a viewport whose <canvas>
// OrbitControls drives, with a settings popover over the canvas.
//
// window.__shell.frame() is one animation frame (controls.update());
// window.__shell.snapshot() returns what tests/fidelity_shell.rs compares:
// every element of the app (its getBoundingClientRect, attributes and
// text), the focused element, the selected object, the camera, the
// controls, and the controls' events since the last snapshot.

import { PerspectiveCamera } from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const BORDER =
  'border: 1px solid #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px';
const CSS = {
  app: 'position: relative; width: 960px; height: 600px; overflow: hidden; background-color: #f8f9fa; font-family: sans-serif; font-size: 12px; line-height: 16px; color: #1e1e1e',
  toolbar:
    'position: absolute; left: 0px; top: 0px; box-sizing: border-box; width: 960px; height: 40px; display: flex; column-gap: 4px; padding-top: 6px; padding-left: 8px; border-bottom: 1px solid #dddddd; background-color: #ffffff',
  toolButton: `width: 88px; height: 28px; box-sizing: border-box; margin: 0px; padding: 0px; ${BORDER}`,
  status: 'width: 120px; height: 28px; padding-left: 8px; line-height: 28px; white-space: nowrap; overflow: hidden',
  objects:
    'position: absolute; left: 0px; top: 40px; box-sizing: border-box; width: 200px; height: 560px; padding: 8px; background-color: #ffffff',
  row: 'box-sizing: border-box; height: 24px; padding-left: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
  splitter: 'position: absolute; left: 200px; top: 40px; width: 6px; height: 560px; background-color: #dddddd',
  viewport:
    'position: absolute; left: 206px; right: 0px; top: 40px; bottom: 0px; overflow: hidden; background-color: #20232a',
  canvas: 'display: block',
  settings:
    'position: absolute; top: 8px; right: 8px; display: none; box-sizing: border-box; width: 184px; padding: 8px; border: 1px solid #dddddd; border-radius: 8px; background-color: #ffffff',
  title: 'height: 24px; margin-bottom: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
  speedRow: 'display: flex; column-gap: 4px; height: 24px; margin-bottom: 4px',
  speedLabel: 'width: 56px; height: 24px; line-height: 24px; white-space: nowrap',
  speedButton: `width: 32px; height: 24px; box-sizing: border-box; margin: 0px; padding: 0px; ${BORDER}`,
  wideButton: `display: block; width: 166px; height: 24px; margin-bottom: 4px; box-sizing: border-box; margin-top: 0px; margin-left: 0px; margin-right: 0px; padding: 0px; ${BORDER}`,
};
const PRESSED = '#e0dfff';
const WHITE = '#ffffff';
const SPLITTER = 6;
const MIN_PANE = 140;
const MAX_PANE = 360;
// [id, label, position]
const OBJECTS = [
  ['origin', 'Origin', [0, 0, 0]],
  ['box', 'Box', [2, 0.5, -1]],
  ['sphere', 'Sphere', [-1.5, 1, 1.5]],
];
// [id, label, rotateSpeed]
const SPEEDS = [
  ['speed-half', '0.5x', 0.5],
  ['speed-1', '1x', 1],
  ['speed-2', '2x', 2],
];
const ATTRIBUTES = [
  'role',
  'aria-label',
  'aria-pressed',
  'aria-selected',
  'aria-expanded',
  'aria-orientation',
  'aria-valuenow',
  'title',
  'type',
  'width',
  'height',
];

function el(tag, css, attributes = {}, text = '') {
  const element = document.createElement(tag);
  element.style.cssText = css;
  for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
  if (text) element.textContent = text;
  return element;
}

function setPressed(button, pressed) {
  button.setAttribute('aria-pressed', String(pressed));
  button.style.backgroundColor = pressed ? PRESSED : WHITE;
}

// ---- The DOM ----

const app = el('div', CSS.app, { id: 'app' });

const toolbar = el('div', CSS.toolbar, { id: 'toolbar', role: 'toolbar', 'aria-label': 'View' });
const reset = el('button', CSS.toolButton, { id: 'reset', type: 'button' }, 'Reset');
const autoRotate = el('button', CSS.toolButton, { id: 'auto-rotate', type: 'button', 'aria-pressed': 'false' }, 'Auto-rotate');
const damping = el('button', CSS.toolButton, { id: 'damping', type: 'button', 'aria-pressed': 'false' }, 'Damping');
const settingsButton = el(
  'button',
  CSS.toolButton,
  { id: 'settings-button', type: 'button', 'aria-expanded': 'false' },
  'Settings',
);
const status = el('div', CSS.status, { id: 'status', role: 'status' }, 'Idle');
for (const element of [reset, autoRotate, damping, settingsButton, status]) toolbar.appendChild(element);
app.appendChild(toolbar);

const objects = el('div', CSS.objects, { id: 'objects', role: 'listbox', 'aria-label': 'Objects' });
const rows = OBJECTS.map(([id, label]) => {
  const row = el('div', CSS.row, { id: `object-${id}`, role: 'option', 'aria-selected': 'false' }, label);
  objects.appendChild(row);
  return [row, id];
});
app.appendChild(objects);

const splitter = el('div', CSS.splitter, {
  id: 'splitter',
  role: 'separator',
  'aria-orientation': 'vertical',
  'aria-label': 'Resize panes',
  'aria-valuenow': '200',
});
app.appendChild(splitter);

const viewport = el('div', CSS.viewport, { id: 'viewport' });
const canvas = el('canvas', CSS.canvas, { id: 'view' });
viewport.appendChild(canvas);

const settings = el('div', CSS.settings, { id: 'settings', role: 'dialog', 'aria-label': 'Settings' });
settings.appendChild(el('div', CSS.title, { id: 'settings-title' }, 'Settings'));
const speedRow = el('div', CSS.speedRow, { id: 'speed-row' });
speedRow.appendChild(el('div', CSS.speedLabel, {}, 'Rotate'));
const speedButtons = SPEEDS.map(([id, label, speed]) => {
  const button = el('button', CSS.speedButton, { id, type: 'button', 'aria-pressed': 'false' }, label);
  speedRow.appendChild(button);
  return [button, speed];
});
settings.appendChild(speedRow);
const screenSpace = el(
  'button',
  CSS.wideButton,
  { id: 'screen-space', type: 'button', 'aria-pressed': 'false' },
  'Screen-space pan',
);
settings.appendChild(screenSpace);
const close = el('button', CSS.wideButton, { id: 'close-settings', type: 'button' }, 'Close');
settings.appendChild(close);
viewport.appendChild(settings);
app.appendChild(viewport);
document.body.appendChild(app);

// ---- The camera and its controls ----

const camera = new PerspectiveCamera(50, 1, 0.1, 100);
camera.position.set(4, 3, 6);
const controls = new OrbitControls(camera, canvas);
controls.listenToKeyEvents(window);
const events = [];
for (const type of ['change', 'start', 'end']) {
  controls.addEventListener(type, () => {
    events.push(type);
    if (type === 'start') status.textContent = 'Moving';
    if (type === 'end') status.textContent = 'Idle';
  });
}

let selected = 'origin';
let settingsOpen = false;
let paneWidth = 200;
let drag = null;

function showSelection() {
  for (const [row, id] of rows) {
    row.setAttribute('aria-selected', String(id === selected));
    row.style.backgroundColor = id === selected ? PRESSED : WHITE;
  }
}

function openSettings(open) {
  settingsOpen = open;
  settings.style.display = open ? 'block' : 'none';
  settingsButton.setAttribute('aria-expanded', String(open));
  settingsButton.style.backgroundColor = open ? PRESSED : WHITE;
}

function setPane(width) {
  paneWidth = width;
  objects.style.width = `${width}px`;
  splitter.style.left = `${width}px`;
  viewport.style.left = `${width + SPLITTER}px`;
  splitter.setAttribute('aria-valuenow', String(width));
}

// div-gap: W17 WebGPU canvas. No renderer draws yet, on either side; this is
// what renderer.setSize(width, height) does to the canvas at pixel ratio 1.
function setSize(width, height) {
  canvas.width = Math.floor(width);
  canvas.height = Math.floor(height);
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
}

setPressed(autoRotate, controls.autoRotate);
setPressed(damping, controls.enableDamping);
setPressed(screenSpace, controls.screenSpacePanning);
for (const [button, speed] of speedButtons) setPressed(button, speed === controls.rotateSpeed);
showSelection();

new ResizeObserver(() => {
  const { width, height } = viewport.getBoundingClientRect();
  camera.aspect = width / height;
  camera.updateProjectionMatrix();
  setSize(width, height);
}).observe(viewport);

// ---- Input ----

reset.addEventListener('click', () => controls.reset());
autoRotate.addEventListener('click', () => {
  controls.autoRotate = !controls.autoRotate;
  setPressed(autoRotate, controls.autoRotate);
});
damping.addEventListener('click', () => {
  controls.enableDamping = !controls.enableDamping;
  setPressed(damping, controls.enableDamping);
});
settingsButton.addEventListener('click', () => openSettings(!settingsOpen));
for (const [button, speed] of speedButtons) {
  button.addEventListener('click', () => {
    controls.rotateSpeed = speed;
    for (const [other, s] of speedButtons) setPressed(other, s === speed);
  });
}
screenSpace.addEventListener('click', () => {
  controls.screenSpacePanning = !controls.screenSpacePanning;
  setPressed(screenSpace, controls.screenSpacePanning);
});
close.addEventListener('click', () => openSettings(false));

for (const [row, id] of rows) {
  row.addEventListener('click', () => {
    selected = id;
    showSelection();
    controls.target.set(...OBJECTS.find((o) => o[0] === id)[2]);
    controls.update();
  });
}

splitter.addEventListener('pointerdown', (event) => {
  if (event.button === 0) drag = { x: event.clientX, width: paneWidth };
});
document.addEventListener('pointermove', (event) => {
  if (drag) setPane(Math.min(MAX_PANE, Math.max(MIN_PANE, drag.width + event.clientX - drag.x)));
});
document.addEventListener('pointerup', (event) => {
  if (drag && event.button === 0) drag = null;
});
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && settingsOpen) openSettings(false);
});

// ---- What the fidelity test reads ----

window.__shell = {
  ready: true,
  settle: () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done)))),
  frame() {
    controls.update();
  },
  snapshot() {
    // Every element of the app, named by its id, or by its parent's name
    // and its index among the parent's elements.
    const elements = [];
    const walk = (node, name) => {
      const r = node.getBoundingClientRect();
      const attributes = {};
      for (const attribute of ATTRIBUTES) {
        if (node.hasAttribute(attribute)) attributes[attribute] = node.getAttribute(attribute);
      }
      const entry = { name, rect: [r.x, r.y, r.width, r.height], attributes };
      if (node.children.length === 0) entry.text = node.textContent;
      elements.push(entry);
      [...node.children].forEach((child, index) => walk(child, child.id || `${name}/${index}`));
    };
    walk(app, 'app');
    const active = document.activeElement;
    return {
      elements,
      focus: active && active !== document.body ? active.id || active.tagName.toLowerCase() : null,
      selected,
      camera: {
        position: camera.position.toArray(),
        quaternion: camera.quaternion.toArray(),
        aspect: camera.aspect,
        zoom: camera.zoom,
        projection: [...camera.projectionMatrix.elements],
      },
      controls: {
        target: controls.target.toArray(),
        state: controls.state,
        autoRotate: controls.autoRotate,
        enableDamping: controls.enableDamping,
        rotateSpeed: controls.rotateSpeed,
        screenSpacePanning: controls.screenSpacePanning,
      },
      events: events.splice(0),
    };
  },
};
