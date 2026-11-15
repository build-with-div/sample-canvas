// Renders samples/three-render/tests/fixtures/scene.json with three.js
// 0.187.0's WebGPURenderer, the way samples/three-render's port does: each
// view into a RenderTarget, read back with readRenderTargetPixelsAsync.
//
// `window.renderScene(scene)` resolves to { adapter, geometry, views }:
// - geometry: each object's attributes and index, as FNV-1a 64 hashes of
//   their bytes;
// - views: each view's RGBA bytes, base64.

import * as THREE from 'three/webgpu';

// FNV-1a, 64-bit, over the bytes of a typed array.
function fnv1a64(array) {
  const bytes = new Uint8Array(array.buffer, array.byteOffset, array.byteLength);
  let hash = 0xcbf29ce484222325n;
  for (const byte of bytes) hash = ((hash ^ BigInt(byte)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return hash.toString(16).padStart(16, '0');
}

function base64(bytes) {
  let text = '';
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(text);
}

function geometryOf({ type, args }) {
  if (type === 'box') return new THREE.BoxGeometry(...args);
  if (type === 'sphere') return new THREE.SphereGeometry(...args);
  throw new Error(`unknown geometry ${type}`);
}

async function renderScene(spec) {
  const renderer = new THREE.WebGPURenderer({ antialias: false });
  await renderer.init();
  if (!renderer.backend.isWebGPUBackend) throw new Error('three.js fell back to WebGL');
  const info = renderer.backend.device.adapterInfo;

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(spec.background);
  const geometry = {};
  for (const object of spec.objects) {
    const g = geometryOf(object.geometry);
    const mesh = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: object.color }));
    mesh.position.fromArray(object.position);
    mesh.rotation.set(...object.rotation);
    scene.add(mesh);
    geometry[object.name] = {
      index: fnv1a64(g.index.array),
      indexType: g.index.array.constructor.name,
      position: fnv1a64(g.attributes.position.array),
      normal: fnv1a64(g.attributes.normal.array),
      uv: fnv1a64(g.attributes.uv.array),
    };
  }

  const views = {};
  for (const view of spec.views) {
    const { fov, near, far } = spec.camera;
    const camera = new THREE.PerspectiveCamera(fov, view.camera.aspect, near, far);
    camera.position.fromArray(view.camera.position);
    camera.quaternion.fromArray(view.camera.quaternion);
    camera.zoom = view.camera.zoom;
    camera.updateProjectionMatrix();
    const target = new THREE.RenderTarget(view.width, view.height);
    renderer.setRenderTarget(target);
    await renderer.renderAsync(scene, camera);
    const data = await renderer.readRenderTargetPixelsAsync(target, 0, 0, view.width, view.height);
    views[view.name] = base64(new Uint8Array(data.buffer, data.byteOffset, view.width * view.height * 4));
    renderer.setRenderTarget(null);
    target.dispose();
  }
  renderer.dispose();
  return {
    adapter: { vendor: info.vendor, architecture: info.architecture, isFallbackAdapter: info.isFallbackAdapter },
    geometry,
    views,
  };
}

window.renderScene = renderScene;
