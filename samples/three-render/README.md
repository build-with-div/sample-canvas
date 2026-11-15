# three-render

A subset of [three.js](https://threejs.org) 0.187.0's `WebGPURenderer`,
on wgpu (milestone P2-6c). It draws the three.js shell's scene offscreen
and reads it back.

What the subset covers:

- **The scene** (in [samples/three](../three)):
  - `Color`, with color management from sRGB to the linear working space;
  - `BoxGeometry` and `SphereGeometry`, with positions, normals, UVs and
    16- or 32-bit indices;
  - a `Scene` of `Mesh`es with `MeshBasicMaterial`, and their world
    matrices.
- **The renderer** (`src/lib.rs`):
  - the scene background as the clear color;
  - the opaque render list sorted by `painterSortStable`, on each mesh's
    bounding-sphere center in clip space;
  - the camera switched to WebGPU's clip space;
  - the `render` and `object` uniform groups as `f32`;
  - `renderPipeline_MeshBasicMaterial`: back faces culled, `less-equal`
    depth on `depth24plus`;
  - a `RenderTarget` (`rgba8unorm`) and `readRenderTargetPixelsAsync`.
- **The shaders** (`src/wgsl`): the WGSL that three.js's node builder
  generates for this material, recorded from three.js in Chromium by
  `tools/three-render-reference/record_wgsl.mjs`. The node system (TSL)
  that writes them is not ported.

Not ported:
- Lights and the other materials.
- Textures.
- Frustum culling, which only skips meshes that draw no pixels.
- Transparency.
- Rendering to a `<canvas>` with its output color-space pass.

Div's W17 scaffolding:

| Platform feature | Div today | Here |
| --- | --- | --- |
| `navigator.gpu` (W17, gap 14) | Div owns no wgpu instance or device | The renderer takes a device its caller creates (`// div-gap: W17`) |
| `GPUCanvasContext` (W17, gap 13) | Neither presenter can composite a GPU texture | Renders into a `RenderTarget` and reads it back; the shell's `<canvas>` stays blank |

## Fidelity

`tests/fidelity_scene.rs` compares the port with three.js 0.187.0's own
`WebGPURenderer` in Chrome for Testing 147.0.7727.50. Both render the scene
in `tests/fixtures/scene.json` from 31 views: every distinct camera state
and canvas size that the shell's Chromium reference recorded (P2-6b).

Both sides render on SwiftShader:
- Chromium through Dawn, with the flags in `tools/webgpu-oracle/flags.mjs`;
- the port through wgpu 30.0.2's Vulkan backend, on the SwiftShader Vulkan
  driver that ships with that Chrome for Testing build.

SwiftShader's output in Chromium is byte-stable from run to run
([docs/webgpu-oracle.md](../../docs/webgpu-oracle.md)), so the comparison is
exact, with no tolerance. The test also checks each geometry's attributes
and index against three.js's (FNV-1a hashes of their bytes). See
[docs/phase2-plan.md](../../docs/phase2-plan.md) (P2-6c) for the
measurements.

To run it, install the driver and point the Vulkan loader at it:

```
python -I tools/webgpu-oracle/install_swiftshader.py out/swiftshader
VK_DRIVER_FILES=out/swiftshader/vk_swiftshader_icd.json cargo test -p three-render -- --nocapture
```

On Windows, also put `out/swiftshader` on `PATH` if the system has no
`vulkan-1.dll`. On Linux, install the Vulkan loader (`libvulkan1`).

Without a SwiftShader adapter the test checks only the geometry, and
prints that it skipped the pixels. On CI (`CI` set) it fails instead.

To recapture the reference, with `CHROME` the `chrome` binary of Chrome for
Testing 147.0.7727.50 and three.js installed by
`npm ci --prefix tools/three-oracle`:

```
python -I tools/three-render-reference/make_scene.py
node tools/three-render-reference/capture.mjs CHROME
node tools/three-render-reference/record_wgsl.mjs CHROME
```
