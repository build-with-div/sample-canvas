// The scenes tools/webgpu-oracle renders in Chromium. Each one draws into a
// WebGPU texture, copies it into a buffer and returns the bytes' SHA-256
// (and, on request, the bytes), so the comparison sees the rasterizer's
// output, not a screenshot of the composited page. Bytes are RGBA: a canvas
// whose preferred format is bgra8unorm is swapped to RGBA after the copy.
//
// - `adapterInfo(options)`: what `navigator.gpu.requestAdapter(options)`
//   gives this browser.
// - `canvasReadback()`: whether a cleared canvas texture can be copied
//   into a buffer and mapped.
// - `renderScene(name, { bytes })`: renders one scene of SCENES.

import * as THREE from 'three/webgpu';

const W = 256;
const H = 256;

async function adapterInfo(options = {}) {
  const out = { secure: isSecureContext, gpu: 'gpu' in navigator };
  if (!navigator.gpu) return out;
  try {
    const adapter = await navigator.gpu.requestAdapter(options);
    out.adapter = adapter !== null;
    if (adapter) {
      out.info = describe(adapter.info);
      out.device = (await adapter.requestDevice()) !== null;
      out.format = navigator.gpu.getPreferredCanvasFormat();
    }
  } catch (error) {
    out.error = String(error);
  }
  return out;
}

function describe(info = {}) {
  return {
    vendor: info.vendor,
    architecture: info.architecture,
    device: info.device,
    description: info.description,
    isFallbackAdapter: info.isFallbackAdapter,
  };
}

async function device() {
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) throw new Error('no WebGPU adapter');
  return { adapter: describe(adapter.info), device: await adapter.requestDevice() };
}

// A canvas configured as three.js configures its own: the preferred format,
// with COPY_SRC so the current texture can be read back.
function canvasContext(device, alphaMode) {
  const canvas = document.createElement('canvas');
  canvas.width = W;
  canvas.height = H;
  document.body.append(canvas);
  const context = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({
    device,
    format,
    alphaMode,
    usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC,
  });
  return { canvas, context, format };
}

// Copies `texture` (4 bytes a texel) into a buffer and returns the bytes
// without row padding, as RGBA.
async function readTexture(device, texture, width, height, format) {
  const bytesPerRow = Math.ceil((width * 4) / 256) * 256;
  const buffer = device.createBuffer({ size: bytesPerRow * height, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
  const encoder = device.createCommandEncoder();
  encoder.copyTextureToBuffer({ texture }, { buffer, bytesPerRow }, [width, height]);
  device.queue.submit([encoder.finish()]);
  await buffer.mapAsync(GPUMapMode.READ);
  const padded = new Uint8Array(buffer.getMappedRange());
  const bytes = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) bytes.set(padded.subarray(y * bytesPerRow, y * bytesPerRow + width * 4), y * width * 4);
  buffer.unmap();
  buffer.destroy();
  if (format === 'bgra8unorm') {
    for (let i = 0; i < bytes.length; i += 4) [bytes[i], bytes[i + 2]] = [bytes[i + 2], bytes[i]];
  } else if (format !== 'rgba8unorm') {
    throw new Error(`can't read ${format}`);
  }
  return bytes;
}

async function canvasReadback() {
  try {
    const { device: gpu } = await device();
    const { canvas, context, format } = canvasContext(gpu, 'opaque');
    const texture = context.getCurrentTexture();
    const encoder = gpu.createCommandEncoder();
    encoder.beginRenderPass({ colorAttachments: [{ view: texture.createView(), clearValue: [0.1, 0.2, 0.3, 1], loadOp: 'clear', storeOp: 'store' }] }).end();
    gpu.queue.submit([encoder.finish()]);
    const bytes = await readTexture(gpu, texture, W, H, format);
    canvas.remove();
    gpu.destroy();
    return `ok ${format} ${Array.from(bytes.subarray(0, 4)).join(',')}`;
  } catch (error) {
    return String(error);
  }
}

// Triangles with per-vertex colors, at fractional positions so edges cross
// pixel centers at many offsets, overlapping under a depth test.
const TRIANGLES = new Float32Array([
  // x, y, z, r, g, b, a
  -0.913, -0.871, 0.5, 1, 0, 0, 1, 0.737, -0.6123, 0.5, 0, 1, 0, 1, -0.1029, 0.9411, 0.5, 0, 0, 1, 1,
  -0.62, 0.333, 0.3, 1, 1, 0, 0.6, 0.9013, 0.181, 0.7, 0, 1, 1, 0.6, 0.0071, -0.9617, 0.3, 1, 0, 1, 0.6,
  -0.3333, -0.25, 0.1, 1, 1, 1, 0.35, 0.4142, -0.1, 0.9, 0.2, 0.4, 0.8, 0.35, -0.05, 0.7071, 0.1, 0.9, 0.5, 0.1, 0.35,
  0.5, 0.5, 0.2, 0.3, 0.3, 0.3, 1, 0.98, 0.98, 0.2, 0.7, 0.7, 0.7, 1, 0.6, 0.99, 0.2, 0.5, 0.5, 0.5, 1,
]);

const SHADER = /* wgsl */ `
struct V { @builtin(position) position: vec4f, @location(0) color: vec4f };
@vertex fn vs(@location(0) p: vec3f, @location(1) c: vec4f) -> V {
  return V(vec4f(p, 1.0), c);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.color; }

@vertex fn fullscreen(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = array(vec2f(-1.0, -3.0), vec2f(3.0, 1.0), vec2f(-1.0, 1.0));
  return vec4f(p[i], 0.0, 1.0);
}
// Transcendental and geometric built-ins, which implementations may
// compute with different precision.
@fragment fn shade(@builtin(position) q: vec4f) -> @location(0) vec4f {
  let p = q.xy / 256.0;
  let n = normalize(vec3f(p - 0.5, 0.35));
  let l = normalize(vec3f(0.3, 0.7, 0.6));
  let d = max(dot(n, l), 0.0);
  let s = pow(max(dot(reflect(-l, n), vec3f(0.0, 0.0, 1.0)), 0.0), 23.0);
  let w = 0.5 + 0.5 * sin(p.x * 37.0) * cos(p.y * 29.0);
  let e = exp(-4.0 * length(p - vec2f(0.3, 0.6)));
  return vec4f(d * 0.8 + s, w * e, atan2(p.y - 0.5, p.x - 0.5) / 6.2831853 + 0.5, 1.0);
}`;

async function rawScene({ sampleCount = 1, blend = false, shade = false, alphaMode = 'opaque' }) {
  const { adapter, device: gpu } = await device();
  const { canvas, context, format } = canvasContext(gpu, alphaMode);
  const module = gpu.createShaderModule({ code: SHADER });
  const target = { format };
  if (blend) {
    target.blend = {
      color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    };
  }
  const pipeline = gpu.createRenderPipeline({
    layout: 'auto',
    vertex: shade
      ? { module, entryPoint: 'fullscreen' }
      : {
          module,
          entryPoint: 'vs',
          buffers: [
            {
              arrayStride: 28,
              attributes: [
                { shaderLocation: 0, offset: 0, format: 'float32x3' },
                { shaderLocation: 1, offset: 12, format: 'float32x4' },
              ],
            },
          ],
        },
    fragment: { module, entryPoint: shade ? 'shade' : 'fs', targets: [target] },
    primitive: { topology: 'triangle-list' },
    depthStencil: blend || shade ? undefined : { format: 'depth24plus', depthWriteEnabled: true, depthCompare: 'less' },
    multisample: { count: sampleCount },
  });
  const current = context.getCurrentTexture();
  const color =
    sampleCount > 1
      ? { view: gpu.createTexture({ size: [W, H], format, sampleCount, usage: GPUTextureUsage.RENDER_ATTACHMENT }).createView(), resolveTarget: current.createView() }
      : { view: current.createView() };
  const pass = {
    colorAttachments: [{ ...color, clearValue: alphaMode === 'opaque' ? [0.1, 0.2, 0.3, 1] : [0, 0, 0, 0], loadOp: 'clear', storeOp: 'store' }],
  };
  if (!blend && !shade) {
    pass.depthStencilAttachment = {
      view: gpu.createTexture({ size: [W, H], format: 'depth24plus', sampleCount, usage: GPUTextureUsage.RENDER_ATTACHMENT }).createView(),
      depthClearValue: 1,
      depthLoadOp: 'clear',
      depthStoreOp: 'store',
    };
  }
  const encoder = gpu.createCommandEncoder();
  const render = encoder.beginRenderPass(pass);
  render.setPipeline(pipeline);
  if (shade) {
    render.draw(3);
  } else {
    const vertices = gpu.createBuffer({ size: TRIANGLES.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
    gpu.queue.writeBuffer(vertices, 0, TRIANGLES);
    render.setVertexBuffer(0, vertices);
    render.draw(TRIANGLES.length / 7);
  }
  render.end();
  gpu.queue.submit([encoder.finish()]);
  const bytes = await readTexture(gpu, current, W, H, format);
  canvas.remove();
  gpu.destroy();
  return { adapter, width: W, height: H, format, bytes };
}

// The shell's objects (samples/three-shell) under the shell's default
// camera, drawn by three.js's WebGPURenderer into a render target.
async function threeScene({ lit }) {
  const width = 320;
  const height = 192;
  const canvas = document.createElement('canvas');
  document.body.append(canvas);
  const renderer = new THREE.WebGPURenderer({ canvas, antialias: false });
  await renderer.init();
  if (!renderer.backend.isWebGPUBackend) throw new Error('three.js fell back to WebGL');
  renderer.setPixelRatio(1);
  renderer.setSize(width, height, false);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x202830);
  const camera = new THREE.PerspectiveCamera(50, width / height, 0.1, 100);
  camera.position.set(4, 3, 5);
  camera.lookAt(0, 0, 0);
  const material = (color) => (lit ? new THREE.MeshStandardMaterial({ color, roughness: 0.45, metalness: 0.1 }) : new THREE.MeshBasicMaterial({ color }));
  const box = new THREE.Mesh(new THREE.BoxGeometry(1.2, 1.2, 1.2), material(0xd04a3a));
  box.position.set(-1.1, 0, 0.4);
  box.rotation.set(0.3, 0.6, 0);
  const sphere = new THREE.Mesh(new THREE.SphereGeometry(0.8, 32, 16), material(0x3a7fd0));
  sphere.position.set(1.2, 0.1, -0.3);
  const origin = new THREE.Mesh(new THREE.SphereGeometry(0.12, 12, 8), material(0xf0f0f0));
  scene.add(box, sphere, origin);
  if (lit) {
    scene.add(new THREE.AmbientLight(0xffffff, 0.35));
    const sun = new THREE.DirectionalLight(0xffffff, 2.2);
    sun.position.set(3, 5, 2);
    scene.add(sun);
  }
  const target = new THREE.RenderTarget(width, height);
  renderer.setRenderTarget(target);
  await renderer.renderAsync(scene, camera);
  const data = await renderer.readRenderTargetPixelsAsync(target, 0, 0, width, height);
  const bytes = new Uint8Array(data.buffer, data.byteOffset, width * height * 4).slice();
  const adapter = describe(renderer.backend.device.adapterInfo);
  target.dispose();
  renderer.dispose();
  canvas.remove();
  return { adapter, width, height, format: 'rgba8unorm', bytes };
}

const SCENES = {
  triangles: () => rawScene({}),
  msaa4: () => rawScene({ sampleCount: 4 }),
  blend: () => rawScene({ blend: true, alphaMode: 'premultiplied' }),
  shading: () => rawScene({ shade: true }),
  'three-basic': () => threeScene({ lit: false }),
  'three-lit': () => threeScene({ lit: true }),
};

function base64(bytes) {
  let text = '';
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(text);
}

async function renderScene(name, { bytes = false } = {}) {
  const result = await SCENES[name]();
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', result.bytes));
  const out = {
    adapter: result.adapter,
    width: result.width,
    height: result.height,
    format: result.format,
    sha256: Array.from(digest, (b) => b.toString(16).padStart(2, '0')).join(''),
  };
  if (bytes) out.bytes = base64(result.bytes);
  return out;
}

window.sceneNames = () => Object.keys(SCENES);
window.adapterInfo = adapterInfo;
window.canvasReadback = canvasReadback;
window.renderScene = renderScene;
