// The three.js side of the oracle: runs the scripts of
// samples/three/tests/fixtures/cases.json on three.js and returns each
// result, with NaN, the infinities and -0 written as strings. oracle.html
// loads it in pinned Chromium; oracle.mjs drives that page.
//
// OrbitControls listens on a DOM element. Here the element, its document and
// the window are stubs that keep listeners and call them, with the
// clientWidth and clientHeight the case gives; events are plain objects
// with the fields OrbitControls reads. Only the JavaScript engine (V8's
// Math) is Chromium's; real input is P2-6b's.

import {
  Euler,
  Matrix4,
  PerspectiveCamera,
  Quaternion,
  Spherical,
  Vector3,
  WebGLCoordinateSystem,
  WebGPUCoordinateSystem,
} from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

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
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, enc(x)]));
  return v;
}
function dec(v) {
  if (v === 'NaN') return NaN;
  if (v === 'Infinity') return Infinity;
  if (v === '-Infinity') return -Infinity;
  if (v === '-0') return -0;
  if (Array.isArray(v)) return v.map(dec);
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, dec(x)]));
  return v;
}

const v3 = (v) => [v.x, v.y, v.z];
const q4 = (q) => [q.x, q.y, q.z, q.w];
const e3 = (e) => [e.x, e.y, e.z];
const m16 = (m) => [...m.elements];
const matrixOf = (a) => new Matrix4().fromArray(a);
const quatOf = (a) => new Quaternion(a[0], a[1], a[2], a[3]);
const vecOf = (a) => new Vector3(a[0], a[1], a[2]);

function mathOp([kind, ...a]) {
  switch (kind) {
    case 'eulerToQuat':
      return q4(new Quaternion().setFromEuler(new Euler(a[0], a[1], a[2], a[3])));
    case 'quatToEuler':
      return e3(new Euler().setFromQuaternion(new Quaternion(a[0], a[1], a[2], a[3]), a[4]));
    case 'eulerRoundTrip': {
      const q = new Quaternion().setFromEuler(new Euler(a[0], a[1], a[2], a[3]));
      return [q4(q), e3(new Euler().setFromQuaternion(q, a[3]))];
    }
    case 'lookAtMatrix':
      return m16(new Matrix4().lookAt(vecOf(a[0]), vecOf(a[1]), vecOf(a[2])));
    case 'quatFromRotation': {
      const m = new Matrix4().makeRotationFromQuaternion(quatOf(a[0]));
      return [m16(m), q4(new Quaternion().setFromRotationMatrix(m))];
    }
    case 'unitVectors':
      return q4(new Quaternion().setFromUnitVectors(vecOf(a[0]), vecOf(a[1])));
    case 'composeDecompose': {
      const m = new Matrix4().compose(vecOf(a[0]), quatOf(a[1]), vecOf(a[2]));
      const p = new Vector3();
      const q = new Quaternion();
      const s = new Vector3();
      m.decompose(p, q, s);
      return [m16(m), v3(p), q4(q), v3(s)];
    }
    case 'invert':
      return m16(matrixOf(a[0]).invert());
    case 'multiply':
      return m16(new Matrix4().multiplyMatrices(matrixOf(a[0]), matrixOf(a[1])));
    case 'spherical': {
      const s = new Spherical().setFromVector3(vecOf(a[0]));
      const raw = [s.radius, s.phi, s.theta];
      s.makeSafe();
      return [raw, s.phi];
    }
    case 'fromSpherical':
      return v3(new Vector3().setFromSphericalCoords(a[0], a[1], a[2]));
    case 'applyQuat':
      return v3(vecOf(a[0]).applyQuaternion(quatOf(a[1])));
    case 'clampLength':
      return v3(vecOf(a[0]).clampLength(a[1], a[2]));
    case 'transformDirection':
      return v3(vecOf(a[0]).transformDirection(matrixOf(a[1])));
    default:
      throw new Error(`unknown math op ${kind}`);
  }
}

const projection = (c) => ({ projection: m16(c.projectionMatrix), inverse: m16(c.projectionMatrixInverse) });
const orientation = (c) => ({ quaternion: q4(c.quaternion), rotation: e3(c.rotation) });

function cameraOp(state, [kind, ...a]) {
  const c = state.camera;
  switch (kind) {
    case 'camera':
      state.camera = new PerspectiveCamera(a[0], a[1], a[2], a[3]);
      return projection(state.camera);
    case 'cset':
      if (a[0] === 'coordinateSystem') c.coordinateSystem = a[1] === 'webgpu' ? WebGPUCoordinateSystem : WebGLCoordinateSystem;
      else c[a[0]] = a[1];
      return {};
    case 'cupdate':
      c.updateProjectionMatrix();
      return projection(c);
    case 'cview':
      c.setViewOffset(...a);
      return { aspect: c.aspect, ...projection(c) };
    case 'cclearView':
      c.clearViewOffset();
      return projection(c);
    case 'cfocal':
      c.setFocalLength(a[0]);
      return { fov: c.fov, ...projection(c) };
    case 'ceffective':
      return { effective: c.getEffectiveFOV(), filmWidth: c.getFilmWidth(), filmHeight: c.getFilmHeight() };
    case 'cpos':
      c.position.set(a[0], a[1], a[2]);
      return {};
    case 'cup':
      c.up.set(a[0], a[1], a[2]);
      return {};
    case 'crot':
      c.rotation.set(a[0], a[1], a[2], a[3]);
      return orientation(c);
    case 'clookAt':
      c.lookAt(a[0], a[1], a[2]);
      return { ...orientation(c), world: m16(c.matrixWorld), worldInverse: m16(c.matrixWorldInverse) };
    default:
      throw new Error(`unknown camera op ${kind}`);
  }
}

class Target {
  constructor() {
    this.listeners = {};
  }
  addEventListener(type, f) {
    (this.listeners[type] ??= []).push(f);
  }
  removeEventListener(type, f) {
    this.listeners[type] = (this.listeners[type] ?? []).filter((g) => g !== f);
  }
  fire(type, event) {
    for (const f of [...(this.listeners[type] ?? [])]) f(event);
  }
}

const noop = () => {};

function orbitState(s, extra) {
  const { controls, camera } = s;
  const events = s.events.splice(0);
  return {
    state: controls.state,
    position: v3(camera.position),
    quaternion: q4(camera.quaternion),
    rotation: e3(camera.rotation),
    target: v3(controls.target),
    matrix: m16(camera.matrix),
    worldInverse: m16(camera.matrixWorldInverse),
    polar: controls.getPolarAngle(),
    azimuth: controls.getAzimuthalAngle(),
    distance: controls.getDistance(),
    zoom: camera.zoom,
    events,
    ...extra,
  };
}

function pointer(button, x, y, mods = {}) {
  return {
    pointerId: 1,
    pointerType: 'mouse',
    button,
    clientX: x,
    clientY: y,
    pageX: x,
    pageY: y,
    ctrlKey: !!mods.ctrl,
    metaKey: !!mods.meta,
    shiftKey: !!mods.shift,
    preventDefault: noop,
  };
}

const MOUSE_ACTIONS = [0, 1, 2];

function orbitOp(s, [kind, ...a]) {
  switch (kind) {
    case 'orbit': {
      const config = a[0];
      const camera = new PerspectiveCamera(config.fov, config.aspect, config.near, config.far);
      camera.position.set(...config.position);
      camera.up.set(...config.up);
      const document = new Target();
      const element = Object.assign(new Target(), {
        clientWidth: config.width,
        clientHeight: config.height,
        style: {},
        ownerDocument: document,
        setPointerCapture: noop,
        releasePointerCapture: noop,
      });
      const window = new Target();
      s.events = [];
      s.camera = camera;
      s.document = document;
      s.element = element;
      s.window = window;
      const controls = new OrbitControls(camera, element);
      for (const type of ['change', 'start', 'end']) controls.addEventListener(type, () => s.events.push(type));
      controls.listenToKeyEvents(window);
      s.controls = controls;
      return orbitState(s);
    }
    case 'set': {
      const [prop, value] = a;
      if (prop === 'target' || prop === 'cursor') s.controls[prop].set(...value);
      else if (prop === 'mouseButtons') {
        const [l, m, r] = value.map((v) => (v === null ? null : MOUSE_ACTIONS[v]));
        s.controls.mouseButtons = { LEFT: l, MIDDLE: m, RIGHT: r };
      } else s.controls[prop] = value;
      return {};
    }
    case 'down':
      s.element.fire('pointerdown', pointer(a[0], a[1], a[2], a[3]));
      return orbitState(s);
    case 'move':
      s.document.fire('pointermove', pointer(0, a[0], a[1], a[2]));
      return orbitState(s);
    case 'up':
      s.document.fire('pointerup', pointer(0, a[0], a[1]));
      return orbitState(s);
    case 'wheel':
      s.element.fire('wheel', { clientX: a[0], clientY: a[1], deltaY: a[2], deltaMode: a[3], ctrlKey: a[4], preventDefault: noop });
      return orbitState(s);
    case 'key': {
      const mods = a[1];
      s.window.fire('keydown', { code: a[0], key: a[0], ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, shiftKey: !!mods.shift, preventDefault: noop });
      return orbitState(s);
    }
    case 'controlKey':
      s.document.fire(a[0] ? 'keydown' : 'keyup', { key: 'Control' });
      return {};
    case 'update':
      return orbitState(s, { returned: s.controls.update(a[0]) });
    case 'rotateLeft':
    case 'rotateUp':
    case 'dollyIn':
    case 'dollyOut':
      s.controls[kind](a[0]);
      return orbitState(s);
    case 'pan':
      s.controls.pan(a[0], a[1]);
      return orbitState(s);
    case 'save':
      s.controls.saveState();
      return {};
    case 'reset':
      s.controls.reset();
      return orbitState(s);
    default:
      throw new Error(`unknown orbit op ${kind}`);
  }
}

export function runCases(cases) {
  return cases.groups.map((group) => {
    const state = {};
    const results = group.ops.map((op) => {
      const [kind, ...args] = dec(op);
      if (group.name === 'math') return enc(mathOp([kind, ...args]));
      if (group.name === 'camera') return enc(cameraOp(state, [kind, ...args]));
      return enc(orbitOp(state, [kind, ...args]));
    });
    return { name: group.name, results };
  });
}
