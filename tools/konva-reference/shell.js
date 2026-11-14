// The Konva demo shell, as a web page: the reference the Div shell in
// samples/konva-shell is compared with. It builds the same DOM with the same
// inline styles and runs the same app code, on Konva 10.9.1 itself drawing
// into real <canvas> elements: a stage in a container, a layers panel, an
// inspector and a toolbar.
//
// window.__shell.settle() waits for the batched draws; window.__shell.snapshot()
// returns what tests/fidelity_shell.rs compares: every element of the app
// (its getBoundingClientRect, attributes and text), the focused element, the
// selection, Konva's node tree and attributes, and the 2D context calls each
// layer canvas received since the last snapshot.
'use strict';

// ---- Harness: record the 2D context calls, seed Math.random ----

// Every context Konva asks for records the calls and assignments it gets,
// as [method, ...args] and ['=property', value], and forwards them to the
// real context. Reads go to the real context, so Konva reads what Chromium
// reports.
const getContext = HTMLCanvasElement.prototype.getContext;
HTMLCanvasElement.prototype.getContext = function (...args) {
  const context = getContext.apply(this, args);
  if (!context || args[0] !== '2d') return context;
  const calls = [];
  this.__calls = calls;
  return new Proxy(context, {
    get(target, key) {
      const value = target[key];
      if (typeof value !== 'function') return value;
      return (...callArgs) => {
        calls.push([key, ...callArgs.map((a) => (Array.isArray(a) ? a.slice() : a))]);
        return value.apply(target, callArgs);
      };
    },
    set(target, key, value) {
      calls.push(['=' + key, value]);
      target[key] = value;
      return true;
    },
  });
};

// Konva picks each shape's hit color with Math.random; the test seeds the
// port's with the same generator.
function mulberry32(seed) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
Math.random = mulberry32(1);

// ---- The app (samples/konva-shell/src/shell.rs) ----

// Konva draws a layer when the app asks (batchDraw), not on every
// attribute change: the port has no automatic drawing.
Konva.autoDrawEnabled = false;

const CSS = {
  app: 'position: relative; width: 960px; height: 600px; overflow: hidden; background-color: #f8f9fa; font-family: sans-serif; font-size: 12px; line-height: 16px; color: #1e1e1e',
  layers:
    'position: absolute; left: 16px; top: 16px; box-sizing: border-box; width: 200px; padding: 4px; border-width: 1px; border-style: solid; border-color: #dddddd; border-radius: 8px; background-color: #ffffff',
  rows: [
    'box-sizing: border-box; height: 24px; padding-left: 8px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
    'box-sizing: border-box; height: 24px; padding-left: 24px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
    'box-sizing: border-box; height: 24px; padding-left: 40px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
  ],
  toolbar: 'position: absolute; left: 232px; top: 16px; display: flex; column-gap: 4px; height: 28px',
  toolButton:
    'box-sizing: border-box; width: 56px; height: 28px; margin: 0px; padding: 0px; border-width: 1px; border-style: solid; border-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px',
  stage:
    'position: absolute; left: 232px; top: 56px; width: 480px; height: 360px; border-width: 1px; border-style: solid; border-color: #dddddd; background-color: #ffffff',
  inspector:
    'position: absolute; left: 728px; top: 16px; display: none; box-sizing: border-box; width: 216px; padding: 8px; border-width: 1px; border-style: solid; border-color: #dddddd; border-radius: 8px; background-color: #ffffff',
  title: 'height: 24px; margin-bottom: 4px; line-height: 24px; white-space: nowrap; overflow: hidden',
  field: 'display: flex; column-gap: 4px; height: 24px; margin-bottom: 4px',
  fieldLabel: 'width: 56px; height: 24px; line-height: 24px; white-space: nowrap',
  fieldValue: 'width: 72px; height: 24px; line-height: 24px; white-space: nowrap; overflow: hidden',
  stepButton:
    'box-sizing: border-box; width: 24px; height: 24px; margin: 0px; padding: 0px; border-width: 1px; border-style: solid; border-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px',
  visibleButton:
    'box-sizing: border-box; width: 72px; height: 24px; margin: 0px; padding: 0px; border-width: 1px; border-style: solid; border-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px',
};
const PRESSED = '#e0dfff';
const SELECTED_SWATCH = '#4a47b1';
const FILLS = ['#e03131', '#1971c2', '#2f9e44', '#f08c00'];
// [attribute, label, step]
const FIELDS = [
  ['x', 'x', 10],
  ['y', 'y', 10],
  ['rotation', 'Rotation', 15],
  ['opacity', 'Opacity', 0.1],
];
const TOOLS = [
  ['add-rect', 'Rect'],
  ['add-circle', 'Circle'],
  ['raise', 'Up'],
  ['lower', 'Down'],
  ['delete', 'Delete'],
];
const ATTRIBUTES = ['role', 'aria-label', 'aria-pressed', 'aria-selected', 'title', 'type', 'class', 'width', 'height'];

function el(tag, css, attributes = {}, text = '') {
  const element = document.createElement(tag);
  element.style.cssText = css;
  for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
  if (text) element.textContent = text;
  return element;
}

const app = el('div', CSS.app, { id: 'app' });
document.body.appendChild(app);

const toolbar = el('div', CSS.toolbar, { id: 'toolbar', role: 'toolbar', 'aria-label': 'Edit' });
const toolButtons = {};
for (const [id, label] of TOOLS) {
  const button = el('button', CSS.toolButton, { id, type: 'button' }, label);
  toolButtons[id] = button;
  toolbar.appendChild(button);
}
app.appendChild(toolbar);

const container = el('div', CSS.stage, { id: 'stage' });
app.appendChild(container);

const layersPanel = el('div', CSS.layers, { id: 'layers', role: 'listbox', 'aria-label': 'Layers' });
app.appendChild(layersPanel);

const inspector = el('div', CSS.inspector, { id: 'inspector', role: 'group', 'aria-label': 'Inspector' });
const title = el('div', CSS.title, { id: 'inspector-title' });
inspector.appendChild(title);
const values = {};
const fieldButtons = [];
for (const [attr, label] of FIELDS) {
  const field = el('div', CSS.field, { id: `field-${attr}` });
  field.appendChild(el('div', CSS.fieldLabel, {}, label));
  values[attr] = el('div', CSS.fieldValue, { id: `value-${attr}` });
  field.appendChild(values[attr]);
  for (const [sign, text, name] of [[-1, '−', 'dec'], [1, '+', 'inc']]) {
    const button = el('button', CSS.stepButton, { id: `${name}-${attr}`, type: 'button', title: `${name === 'dec' ? 'Decrease' : 'Increase'} ${label}` }, text);
    fieldButtons.push([button, attr, sign]);
    field.appendChild(button);
  }
  inspector.appendChild(field);
}
const fillRow = el('div', CSS.field, { id: 'field-fill' });
const swatches = FILLS.map((fill) => {
  const button = el('button', CSS.stepButton, { id: `fill-${fill.slice(1)}`, type: 'button', title: fill, 'aria-pressed': 'false' });
  button.style.backgroundColor = fill;
  fillRow.appendChild(button);
  return [button, fill];
});
inspector.appendChild(fillRow);
const visibleButton = el('button', CSS.visibleButton, { id: 'toggle-visible', type: 'button', 'aria-pressed': 'true' }, 'Visible');
inspector.appendChild(visibleButton);
app.appendChild(inspector);

// The scene.
const stage = new Konva.Stage({ container, width: 480, height: 360 });
const background = new Konva.Layer({ id: 'background', name: 'Background' });
background.add(new Konva.Rect({ id: 'sky', name: 'Sky', x: 0, y: 0, width: 480, height: 240, fill: '#e7f5ff' }));
background.add(new Konva.Rect({ id: 'ground', name: 'Ground', x: 0, y: 240, width: 480, height: 120, fill: '#ebfbee' }));
const shapes = new Konva.Layer({ id: 'shapes', name: 'Shapes' });
// div-gap: W2 W6. A translucent shape with a fill and a stroke is drawn
// through Konva's isolation canvas, which the port can't draw (it needs the
// context's getTransform and drawImage of a canvas), so stroked shapes turn
// perfectDrawEnabled off, here and in the Div shell.
shapes.add(
  new Konva.Rect({ id: 'red-box', name: 'Red box', x: 60, y: 60, width: 120, height: 80, fill: '#e03131', stroke: '#1e1e1e', strokeWidth: 2, cornerRadius: 8, perfectDrawEnabled: false }),
);
shapes.add(new Konva.Circle({ id: 'blue-dot', name: 'Blue dot', x: 300, y: 120, radius: 40, fill: '#1971c2' }));
const pair = new Konva.Group({ id: 'pair', name: 'Pair', x: 200, y: 220, rotation: 15 });
pair.add(
  new Konva.Rect({ id: 'pair-rect', name: 'Pair rect', x: 0, y: 0, width: 60, height: 40, fill: '#2f9e44', stroke: '#1e1e1e', strokeWidth: 1, dash: [4, 2], perfectDrawEnabled: false }),
);
pair.add(new Konva.Circle({ id: 'pair-dot', name: 'Pair dot', x: 90, y: 20, radius: 20, fill: '#f08c00', opacity: 0.8 }));
shapes.add(pair);
stage.add(background);
stage.add(shapes);

let selected = null;
let created = 0;

function isContainer(node) {
  return node.getClassName() === 'Layer' || node.getClassName() === 'Group';
}

function children(node) {
  return isContainer(node) ? node.getChildren() : [];
}

// Depth first, as Konva orders the tree.
function nodeById(id) {
  const walk = (nodes) => {
    for (const node of nodes) {
      if (node.id() === id) return node;
      const found = walk(children(node));
      if (found) return found;
    }
    return null;
  };
  return walk(stage.getChildren());
}

function redraw(layer) {
  layer.batchDraw();
}

function select(id) {
  selected = id;
  update();
}

function adjust(attr, sign, step) {
  const node = nodeById(selected);
  let value = node[attr]() + sign * step;
  if (attr === 'opacity') value = Math.min(1, Math.max(0, value));
  node[attr](value);
  redraw(node.getLayer());
  update();
}

function setFill(fill) {
  const node = nodeById(selected);
  node.fill(fill);
  redraw(node.getLayer());
  update();
}

function toggleVisible() {
  const node = nodeById(selected);
  node.visible(!node.visible());
  redraw(node.getLayer());
  update();
}

function nudge(dx, dy) {
  const node = selected && nodeById(selected);
  if (!node) return;
  node.x(node.x() + dx);
  node.y(node.y() + dy);
  redraw(node.getLayer());
  update();
}

function add(kind) {
  const node = selected && nodeById(selected);
  const target = !node ? shapes : isContainer(node) ? node : node.getParent();
  created += 1;
  const n = created;
  const fill = FILLS[(n - 1) % FILLS.length];
  const shape =
    kind === 'rect'
      ? new Konva.Rect({ id: `rect-${n}`, name: `Rect ${n}`, x: 20 + 30 * n, y: 20 + 20 * n, width: 80, height: 50, fill })
      : new Konva.Circle({ id: `circle-${n}`, name: `Circle ${n}`, x: 60 + 30 * n, y: 60 + 20 * n, radius: 30, fill });
  target.add(shape);
  selected = shape.id();
  redraw(target.getLayer());
  update();
}

function restack(up) {
  const node = selected && nodeById(selected);
  if (!node || node.getClassName() === 'Layer') return;
  if (up ? node.moveUp() : node.moveDown()) redraw(node.getLayer());
  update();
}

function remove() {
  const node = selected && nodeById(selected);
  if (!node || node.getClassName() === 'Layer') return;
  const layer = node.getLayer();
  node.destroy();
  selected = null;
  redraw(layer);
  update();
}

function update() {
  // The layers panel: topmost first at every level.
  const rows = [];
  const walk = (nodes, depth) => {
    for (const node of nodes.slice().reverse()) {
      const id = node.id();
      const isSelected = id === selected;
      const row = el('div', CSS.rows[depth], { id: `row-${id}`, role: 'option', 'aria-selected': String(isSelected) }, node.name());
      if (isSelected) row.style.backgroundColor = PRESSED;
      row.addEventListener('click', () => select(id));
      rows.push(row);
      walk(children(node), depth + 1);
    }
  };
  walk(stage.getChildren(), 0);
  layersPanel.replaceChildren(...rows);

  const node = selected && nodeById(selected);
  inspector.style.display = node ? 'block' : 'none';
  if (!node) return;
  title.textContent = `${node.getClassName()} ${node.id()}`;
  for (const [attr] of FIELDS) values[attr].textContent = String(node[attr]());
  const isShape = !isContainer(node);
  fillRow.style.display = isShape ? 'flex' : 'none';
  for (const [button, fill] of swatches) {
    const pressed = isShape && node.fill() === fill;
    button.setAttribute('aria-pressed', String(pressed));
    button.style.borderColor = pressed ? SELECTED_SWATCH : '#cccccc';
  }
  const visible = node.visible();
  visibleButton.setAttribute('aria-pressed', String(visible));
  visibleButton.style.backgroundColor = visible ? PRESSED : '#ffffff';
}

toolButtons['add-rect'].addEventListener('click', () => add('rect'));
toolButtons['add-circle'].addEventListener('click', () => add('circle'));
toolButtons.raise.addEventListener('click', () => restack(true));
toolButtons.lower.addEventListener('click', () => restack(false));
toolButtons.delete.addEventListener('click', remove);
for (const [button, attr, sign] of fieldButtons) {
  const step = FIELDS.find(([a]) => a === attr)[2];
  button.addEventListener('click', () => adjust(attr, sign, step));
}
for (const [button, fill] of swatches) button.addEventListener('click', () => setFill(fill));
visibleButton.addEventListener('click', toggleVisible);
document.addEventListener('keydown', (event) => {
  switch (event.key) {
    case 'Escape':
      select(null);
      break;
    case 'Delete':
    case 'Backspace':
      remove();
      break;
    case 'ArrowLeft':
      nudge(-1, 0);
      break;
    case 'ArrowRight':
      nudge(1, 0);
      break;
    case 'ArrowUp':
      nudge(0, -1);
      break;
    case 'ArrowDown':
      nudge(0, 1);
      break;
  }
});
update();

// ---- What the fidelity test reads ----

window.__shell = {
  settle: () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done)))),
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
    const tree = [];
    const walkTree = (node, depth) => {
      tree.push({ depth, className: node.getClassName(), attrs: { ...node.attrs } });
      for (const child of children(node)) walkTree(child, depth + 1);
    };
    for (const layer of stage.getChildren()) walkTree(layer, 0);
    const traces = {};
    for (const layer of stage.getChildren()) {
      traces[`${layer.id()}.scene`] = layer.getCanvas()._canvas.__calls.splice(0);
      traces[`${layer.id()}.hit`] = layer.getHitCanvas()._canvas.__calls.splice(0);
    }
    const active = document.activeElement;
    return {
      elements,
      focus: active && active !== document.body ? active.id || active.tagName.toLowerCase() : null,
      selected,
      tree,
      traces,
    };
  },
};
