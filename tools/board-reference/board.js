// The board, as a web page: the reference the Div shell in samples/board
// is compared with. It builds the same DOM with the same inline styles,
// implements the same scene model (samples/board/src/scene.rs), and draws
// with rough.js 4.6.6 itself on a real <canvas>, as an Excalidraw-style
// board does: pointer events, coordinates from clientX/clientY and
// getBoundingClientRect, and a redraw in requestAnimationFrame.
//
// window.__board.settle() waits for the redraw; window.__board.snapshot()
// returns what tests/fidelity_board.rs compares: every element with an id
// (its getBoundingClientRect and attributes), the focused element, the
// scene, and the 2D context calls of the last redraw.
'use strict';

const CSS = {
  board:
    'position: relative; width: 960px; height: 600px; overflow: hidden; background-color: #ffffff; font-family: sans-serif; font-size: 12px; line-height: 16px; color: #1e1e1e',
  canvas: 'position: absolute; left: 0px; top: 0px; display: block; width: 960px; height: 600px',
  toolbar:
    'position: absolute; left: 377px; top: 16px; display: flex; column-gap: 4px; padding: 4px; border-width: 1px; border-style: solid; border-color: #dddddd; border-radius: 8px; background-color: #ffffff',
  toolButton:
    'box-sizing: border-box; width: 36px; height: 36px; margin: 0px; padding: 0px; border-width: 1px; border-style: solid; border-color: #ffffff; border-radius: 6px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px',
  stylePanel:
    'position: absolute; left: 16px; top: 16px; display: none; box-sizing: border-box; width: 184px; padding: 12px; border-width: 1px; border-style: solid; border-color: #dddddd; border-radius: 8px; background-color: #ffffff',
  section: 'margin-top: 12px',
  firstSection: 'margin-top: 0px',
  label: 'height: 16px; margin-bottom: 6px; white-space: nowrap',
  options: 'display: flex; column-gap: 6px; height: 28px',
  option:
    'box-sizing: border-box; width: 28px; height: 28px; margin: 0px; padding: 0px; border-width: 1px; border-style: solid; border-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px',
  layers:
    'position: absolute; left: 768px; top: 16px; display: none; box-sizing: border-box; width: 176px; padding: 4px; border-width: 1px; border-style: solid; border-color: #dddddd; border-radius: 8px; background-color: #ffffff',
  row: 'height: 28px; padding-left: 8px; padding-right: 8px; border-radius: 4px; line-height: 28px; white-space: nowrap; overflow: hidden',
};
const PRESSED = '#e0dfff';
const SELECTED_SWATCH = '#4a47b1';

const TOOLS = [
  ['selection', 'V', 'Selection — V or 1'],
  ['rectangle', 'R', 'Rectangle — R or 2'],
  ['diamond', 'D', 'Diamond — D or 3'],
  ['ellipse', 'O', 'Ellipse — O or 4'],
  ['line', 'L', 'Line — L or 5'],
];
const TOOL_KEYS = {
  v: 'selection', V: 'selection', 1: 'selection',
  r: 'rectangle', R: 'rectangle', 2: 'rectangle',
  d: 'diamond', D: 'diamond', 3: 'diamond',
  o: 'ellipse', O: 'ellipse', 4: 'ellipse',
  l: 'line', L: 'line', 5: 'line',
};
// [name, label, [[value, label], ...], color section]
const SECTIONS = [
  ['stroke', 'Stroke', [['#1e1e1e', ''], ['#e03131', ''], ['#2f9e44', ''], ['#1971c2', '']], true],
  ['background', 'Background', [['transparent', ''], ['#ffc9c9', ''], ['#b2f2bb', ''], ['#a5d8ff', '']], true],
  ['width', 'Stroke width', [['1', 'S'], ['2', 'M'], ['4', 'L']], false],
  ['roughness', 'Sloppiness', [['0', '0'], ['1', '1'], ['2', '2']], false],
  ['fill', 'Fill', [['hachure', 'H'], ['cross-hatch', 'X'], ['solid', 'S']], false],
];
const ATTRIBUTES = ['role', 'aria-label', 'aria-pressed', 'aria-selected', 'data-tool', 'data-section', 'data-value', 'title', 'type'];

// ---- The scene model (samples/board/src/scene.rs) ----

class Scene {
  constructor() {
    this.elements = [];
    this.selected = null;
    this.tool = 'selection';
    this.style = { strokeColor: '#1e1e1e', backgroundColor: 'transparent', strokeWidth: 2, roughness: 1, fillStyle: 'solid' };
    this.drag = null;
    this.created = 0;
  }

  element(id) {
    return this.elements.find((e) => e.id === id);
  }

  hit(x, y) {
    for (let i = this.elements.length - 1; i >= 0; i--) {
      const e = this.elements[i];
      const [x0, x1] = ordered(e.x, e.x + e.width);
      const [y0, y1] = ordered(e.y, e.y + e.height);
      if (x0 <= x && x <= x1 && y0 <= y && y <= y1) return e;
    }
    return null;
  }

  setTool(tool) {
    this.tool = tool;
    this.selected = null;
  }

  setStyle(section, value) {
    const apply = (style) => {
      if (section === 'stroke') style.strokeColor = value;
      else if (section === 'background') style.backgroundColor = value;
      else if (section === 'width') style.strokeWidth = Number(value);
      else if (section === 'roughness') style.roughness = Number(value);
      else if (section === 'fill') style.fillStyle = value;
    };
    apply(this.style);
    const element = this.selected && this.element(this.selected);
    if (element) apply(element.style);
  }

  select(id) {
    if (this.element(id)) {
      this.tool = 'selection';
      this.selected = id;
    }
  }

  pointerDown(x, y) {
    if (this.tool === 'selection') {
      const hit = this.hit(x, y);
      if (hit) {
        this.selected = hit.id;
        this.drag = { move: hit.id, pointer: [x, y], start: [hit.x, hit.y] };
      } else {
        this.selected = null;
      }
      return;
    }
    const n = ++this.created;
    const id = `el-${n}`;
    const kind = this.tool;
    this.elements.push({
      id,
      type: kind,
      name: `${kind[0].toUpperCase()}${kind.slice(1)} ${n}`,
      x,
      y,
      width: 0,
      height: 0,
      style: { ...this.style },
      seed: n,
    });
    this.selected = id;
    this.drag = { create: id, origin: [x, y] };
  }

  pointerMove(x, y) {
    const drag = this.drag;
    if (!drag) return false;
    if (drag.create) {
      const e = this.element(drag.create);
      const [ox, oy] = drag.origin;
      if (e.type === 'line') {
        e.width = x - ox;
        e.height = y - oy;
      } else {
        e.x = Math.min(ox, x);
        e.y = Math.min(oy, y);
        e.width = Math.abs(x - ox);
        e.height = Math.abs(y - oy);
      }
    } else {
      const e = this.element(drag.move);
      e.x = drag.start[0] + (x - drag.pointer[0]);
      e.y = drag.start[1] + (y - drag.pointer[1]);
    }
    return true;
  }

  pointerUp() {
    const drag = this.drag;
    this.drag = null;
    if (!drag) return false;
    if (drag.create) {
      const e = this.element(drag.create);
      if (e.width === 0 && e.height === 0) {
        this.elements = this.elements.filter((x) => x.id !== e.id);
        this.selected = null;
      } else {
        this.tool = 'selection';
      }
    }
    return true;
  }

  key(key) {
    if (this.drag) return false;
    if (TOOL_KEYS[key]) {
      this.setTool(TOOL_KEYS[key]);
      return true;
    }
    if (key === 'Escape') {
      this.selected = null;
      this.tool = 'selection';
      return true;
    }
    if (key === 'Delete' || key === 'Backspace') {
      if (!this.selected) return false;
      const id = this.selected;
      this.selected = null;
      this.elements = this.elements.filter((e) => e.id !== id);
      return true;
    }
    return false;
  }

  showsStylePanel() {
    return this.tool !== 'selection' || this.selected !== null;
  }
}

function ordered(a, b) {
  return a <= b ? [a, b] : [b, a];
}

function roughOptions(e) {
  const s = e.style;
  const o = {
    seed: e.seed,
    stroke: s.strokeColor,
    strokeWidth: s.strokeWidth,
    roughness: s.roughness,
    hachureGap: s.strokeWidth * 4,
    fillWeight: s.strokeWidth / 2,
    fillStyle: s.fillStyle,
  };
  if (s.backgroundColor !== 'transparent') o.fill = s.backgroundColor;
  return o;
}

function styleValue(style, section) {
  return String(
    { stroke: style.strokeColor, background: style.backgroundColor, width: style.strokeWidth, roughness: style.roughness, fill: style.fillStyle }[section],
  );
}

// ---- The DOM ----

function el(tag, css, attributes = {}, text = '') {
  const node = document.createElement(tag);
  node.style.cssText = css;
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  if (text) node.textContent = text;
  return node;
}

const scene = new Scene();
const board = el('div', CSS.board, { id: 'board' });
const canvas = el('canvas', CSS.canvas, { id: 'scene', width: '960', height: '600' });
board.append(canvas);

const toolbar = el('div', CSS.toolbar, { id: 'toolbar', role: 'toolbar', 'aria-label': 'Tools' });
const toolButtons = TOOLS.map(([name, label, title]) => {
  const button = el('button', CSS.toolButton, { id: `tool-${name}`, type: 'button', 'data-tool': name, title, 'aria-pressed': 'false' }, label);
  button.addEventListener('click', () => {
    scene.setTool(name);
    update();
  });
  toolbar.append(button);
  return button;
});
board.append(toolbar);

const stylePanel = el('div', CSS.stylePanel, { id: 'style-panel', role: 'group', 'aria-label': 'Style' });
const optionButtons = [];
SECTIONS.forEach(([name, label, options, color], index) => {
  const section = el('div', index === 0 ? CSS.firstSection : CSS.section, { id: `section-${name}` });
  section.append(el('div', CSS.label, { id: `label-${name}` }, label));
  const row = el('div', CSS.options, { id: `options-${name}` });
  for (const [value, text] of options) {
    const button = el(
      'button',
      CSS.option,
      { id: `${name}-${value.replace('#', '')}`, type: 'button', 'data-section': name, 'data-value': value, title: value, 'aria-pressed': 'false' },
      text,
    );
    if (color) button.style.backgroundColor = value;
    button.addEventListener('click', () => {
      scene.setStyle(name, value);
      update();
    });
    row.append(button);
    optionButtons.push([button, name, value, color]);
  }
  section.append(row);
  stylePanel.append(section);
});
board.append(stylePanel);

const layers = el('div', CSS.layers, { id: 'layers', role: 'listbox', 'aria-label': 'Elements' });
board.append(layers);
document.body.append(board);

// ---- Rendering ----

// rough.js draws through a Proxy around the real context, which records
// each access as samples/roughjs's CallRecorder does and then performs it.
const real = canvas.getContext('2d');
let trace = [];
let lastTrace = [];
const METHODS = new Set(['save', 'restore', 'beginPath', 'moveTo', 'lineTo', 'bezierCurveTo', 'fill', 'stroke', 'setLineDash']);
const copy = (a) => (Array.isArray(a) ? [...a] : a);
const recording = new Proxy(real, {
  get(target, prop) {
    if (METHODS.has(prop)) {
      return (...args) => {
        trace.push([prop, ...args.map(copy)]);
        return target[prop](...args);
      };
    }
    trace.push(['get ' + String(prop)]);
    const value = target[prop];
    return typeof value === 'function' ? value.bind(target) : value;
  },
  set(target, prop, value) {
    trace.push([String(prop) + '=', copy(value)]);
    target[prop] = value;
    return true;
  },
});
const rc = rough.canvas({ getContext: () => recording });

let frame = 0;
function redraw() {
  frame = 0;
  trace = [];
  real.clearRect(0, 0, canvas.width, canvas.height);
  for (const e of scene.elements) {
    if (e.width === 0 && e.height === 0) continue;
    const o = roughOptions(e);
    const { x, y, width: w, height: h } = e;
    if (e.type === 'rectangle') rc.rectangle(x, y, w, h, o);
    else if (e.type === 'ellipse') rc.ellipse(x + w / 2, y + h / 2, w, h, o);
    else if (e.type === 'diamond') rc.polygon([[x + w / 2, y], [x + w, y + h / 2], [x + w / 2, y + h], [x, y + h / 2]], o);
    else if (e.type === 'line') rc.linearPath([[x, y], [x + w, y + h]], o);
  }
  lastTrace = trace;
}

function update() {
  for (const [i, [name]] of TOOLS.entries()) {
    const pressed = scene.tool === name;
    toolButtons[i].setAttribute('aria-pressed', String(pressed));
    toolButtons[i].style.backgroundColor = pressed ? PRESSED : '#ffffff';
  }
  for (const [button, section, value, color] of optionButtons) {
    const pressed = styleValue(scene.style, section) === value;
    button.setAttribute('aria-pressed', String(pressed));
    if (color) button.style.borderColor = pressed ? SELECTED_SWATCH : '#cccccc';
    else button.style.backgroundColor = pressed ? PRESSED : '#ffffff';
  }
  stylePanel.style.display = scene.showsStylePanel() ? 'block' : 'none';
  layers.style.display = scene.elements.length ? 'block' : 'none';
  layers.replaceChildren(
    ...scene.elements
      .slice()
      .reverse()
      .map((e) => {
        const selected = e.id === scene.selected;
        const row = el('div', CSS.row, { id: `layer-${e.id}`, role: 'option', 'aria-selected': String(selected) }, e.name);
        if (selected) row.style.backgroundColor = PRESSED;
        row.addEventListener('click', () => {
          scene.select(e.id);
          update();
        });
        return row;
      }),
  );
  if (!frame) frame = requestAnimationFrame(redraw);
}

canvas.addEventListener('pointerdown', (event) => {
  if (event.button !== 0) return;
  const rect = canvas.getBoundingClientRect();
  scene.pointerDown(event.clientX - rect.left, event.clientY - rect.top);
  update();
});
window.addEventListener('pointermove', (event) => {
  const rect = canvas.getBoundingClientRect();
  if (scene.pointerMove(event.clientX - rect.left, event.clientY - rect.top)) update();
});
window.addEventListener('pointerup', (event) => {
  if (event.button === 0 && scene.pointerUp()) update();
});
document.addEventListener('keydown', (event) => {
  if (scene.key(event.key)) update();
});
update();

// ---- What the fidelity test reads ----

window.__board = {
  settle: () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done)))),
  snapshot() {
    const elements = [];
    const walk = (node) => {
      if (node.id) {
        const r = node.getBoundingClientRect();
        const attributes = {};
        for (const name of ATTRIBUTES) {
          if (node.hasAttribute(name)) attributes[name] = node.getAttribute(name);
        }
        elements.push({ id: node.id, rect: [r.x, r.y, r.width, r.height], attributes });
      }
      for (const child of node.children) walk(child);
    };
    walk(board);
    const active = document.activeElement;
    return {
      elements,
      focus: active && active !== document.body ? active.id || active.tagName.toLowerCase() : null,
      tool: scene.tool,
      selected: scene.selected,
      scene: scene.elements.map((e) => ({
        id: e.id,
        type: e.type,
        name: e.name,
        x: e.x,
        y: e.y,
        width: e.width,
        height: e.height,
        strokeColor: e.style.strokeColor,
        backgroundColor: e.style.backgroundColor,
        strokeWidth: e.style.strokeWidth,
        roughness: e.style.roughness,
        fillStyle: e.style.fillStyle,
        seed: e.seed,
      })),
      trace: lastTrace,
    };
  },
};
