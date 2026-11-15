# The WebGPU oracle: SwiftShader in pinned Chromium

Route A's pixel comparisons (W17 and milestone P2-6c) need a reference
WebGPU renderer that gives the same bytes every time. The first step of
W17 (decision 5; [design.md](design.md) §6 step 7 and §7.3) is to check
that pinned Chromium has one. This note records that check.

In summary, Chrome for Testing 147.0.7727.50 renders WebGPU on SwiftShader,
its CPU Vulkan implementation, with the flags below. Six scenes, read
straight back from their textures, gave identical bytes across 20 browser
launches on each OS. The Windows and Linux bytes were identical too.

## Setup

- Browser: Chrome for Testing 147.0.7727.50 (`chrome-win64` and
  `chrome-linux64`), headless (`--headless=new`), with the other flags of
  [tools/board-reference/chromium.mjs](../tools/board-reference/chromium.mjs).
- Machine: one PC, a 12th Gen Intel Core i9-12900KF, with an NVIDIA GPU
  (Ampere).
  - Windows: Windows 11 Pro 10.0.26300, Node 22.18.0.
  - Linux: Ubuntu under WSL2 (kernel 6.18.58.1-microsoft-standard-WSL2),
    Node 22.18.0. WSL exposes no GPU to Chromium here.
- Pages: [tools/webgpu-oracle](../tools/webgpu-oracle), served from
  `http://127.0.0.1`. WebGPU needs a secure context, and module scripts
  don't load from `file:`.
- Measurements: [tools/webgpu-oracle/measurements](../tools/webgpu-oracle/measurements).

## Flags

`probe.mjs` launches Chromium once with `--disable-gpu` and once for each of
the 32 subsets of these five flags:

- `--enable-unsafe-webgpu`
- `--use-webgpu-adapter=swiftshader`
- `--enable-features=Vulkan`
- `--use-vulkan=swiftshader`
- `--use-angle=vulkan`

For each launch it records:

- the adapter that `requestAdapter()` returns;
- the adapter that `requestAdapter({ forceFallbackAdapter: true })` returns;
- whether a WebGPU canvas's texture can be cleared, copied into a buffer
  and mapped.

The table summarizes `probe-win32.jsonl` and `probe-linux.jsonl`.

| Flags | Windows | Linux (WSL2) |
| --- | --- | --- |
| `--disable-gpu`, the fidelity runners' default | no adapter | no adapter |
| none | NVIDIA (hardware) | no adapter |
| any set without `--enable-unsafe-webgpu` | NVIDIA or no adapter | no adapter |
| `--enable-unsafe-webgpu` | NVIDIA by default; SwiftShader only with `forceFallbackAdapter` | SwiftShader, but configuring a canvas destroys the device |
| `--enable-unsafe-webgpu --use-webgpu-adapter=swiftshader` | **SwiftShader; canvas readback works** | SwiftShader, but configuring a canvas destroys the device |
| `--enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan` | no adapter | **SwiftShader; canvas readback works** |
| any other set with `--use-angle=vulkan` | no adapter | as without it |

Details:

- **The adapter.** It reports `vendor: "google"`,
  `architecture: "swiftshader"` and `isFallbackAdapter: true`.
- **Windows.** The two Vulkan flags change nothing. `--use-angle=vulkan`
  removes WebGPU entirely.
- **Linux, without the compositor on Vulkan.** The canvas failure looks
  like this:
  - After `configure()`, the device is lost with reason `destroyed`
    ("Device was destroyed.").
  - `mapAsync` rejects with "A valid external Instance reference no longer
    exists."
  - A texture that is not a canvas's can still be read back.
  - Only sets that have both `--enable-features=Vulkan` and
    `--use-angle=vulkan` avoid it.
- **Canvas format.** The preferred canvas format is `bgra8unorm` on Windows
  and `rgba8unorm` on Linux. `scenes.js` swaps BGRA to RGBA after the copy,
  so the two can be compared byte for byte.

So the oracle's flags, in `capture.mjs`, are the smallest sets that pass on
each OS:

- Windows: `--enable-unsafe-webgpu --use-webgpu-adapter=swiftshader`
- Linux: `--enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan`

The probe's canvas readback fills a texture with the clear color
(0.1, 0.2, 0.3, 1). SwiftShader stores it as bytes 26, 51, 77. The NVIDIA
adapter stores 25, 51, 76. Hardware is allowed to convert floats to bytes
this way, which is one reason the oracle must be the software adapter.

## Determinism

`capture.mjs` launches a new browser for each run. Each run renders every
scene twice in the same page. Each render copies the scene's texture into
a buffer, removes the row padding, converts it to RGBA and hashes it
(SHA-256).

| Scene | What it draws | Size |
| --- | --- | --- |
| `triangles` | Overlapping per-vertex-colored triangles at fractional positions, with a depth test, on a `GPUCanvasContext` texture | 256×256 |
| `msaa4` | The same, drawn 4× multisampled and resolved into the canvas texture | 256×256 |
| `blend` | The same triangles, alpha-blended, on a canvas with `alphaMode: "premultiplied"` | 256×256 |
| `shading` | A full-screen fragment shader using `normalize`, `dot`, `reflect`, `pow`, `sin`, `cos`, `exp`, `length` and `atan2` | 256×256 |
| `three-basic` | three.js 0.187.0 `WebGPURenderer`: the shell's box, sphere and origin marker with `MeshBasicMaterial`, under a perspective camera, into a `RenderTarget` (linear, no output color conversion) | 320×192 |
| `three-lit` | The same with `MeshStandardMaterial`, an ambient and a directional light | 320×192 |

20 runs per OS give 40 renders per scene per OS:

| Scene | Windows: distinct hashes | Linux: distinct hashes | Windows vs Linux |
| --- | --- | --- | --- |
| `triangles` | 1 of 40 (`66b247200f764100…`) | 1 of 40, the same | 0 of 65,536 pixels differ |
| `msaa4` | 1 of 40 (`7c8956f3016ced74…`) | 1 of 40, the same | 0 of 65,536 |
| `blend` | 1 of 40 (`1730ce8d36a0b1f1…`) | 1 of 40, the same | 0 of 65,536 |
| `shading` | 1 of 40 (`a6881b91511ccbcf…`) | 1 of 40, the same | 0 of 65,536 |
| `three-basic` | 1 of 40 (`2330ac05a2e70101…`) | 1 of 40, the same | 0 of 61,440 |
| `three-lit` | 1 of 40 (`b1f9744c67fa6be0…`) | 1 of 40, the same | 0 of 61,440 |

So SwiftShader WebGPU in pinned Chromium is byte-stable on this machine,
from render to render, launch to launch, and OS to OS. P2-6c can compare
exactly. There is no Chromium-vs-Chromium variance to derive a tolerance
from.

## Not established

- **Other CPUs.** SwiftShader compiles shaders to machine code at run time
  and picks instructions by CPU feature. Both OSes here ran on the same
  i9-12900KF. Whether a CI runner's CPU gives the same bytes has not been
  measured. Recording the oracle once, as for the other fixtures, avoids
  depending on it.
- **wgpu matching these bytes.** Chromium compiles WGSL with Tint, and wgpu
  with naga, so the same WGSL can reach SwiftShader as different SPIR-V.
  wgpu running on another Vulkan driver, such as lavapipe, also rasterizes
  differently. Both Chrome for Testing builds ship SwiftShader's Vulkan
  driver (`vk_swiftshader.dll` or `libvk_swiftshader.so`, with
  `vk_swiftshader_icd.json`; SwiftShader is Apache-2.0). P2-6c can point
  wgpu's Vulkan backend at that driver, so both sides share a rasterizer.
  Whether the bytes then match is P2-6c's measurement.
- **Device scale factors other than 1, and the composited page.** These
  captures read textures, not screenshots. Composition is §7.3's separate
  comparison.

## Reproducing

With `CHROME` the `chrome` binary of Chrome for Testing 147.0.7727.50, and
three.js installed by `npm ci --prefix tools/three-oracle`:

```
node tools/webgpu-oracle/probe.mjs CHROME > probe.jsonl
node tools/webgpu-oracle/capture.mjs CHROME 20 capture.json dump/
node tools/webgpu-oracle/compare.mjs dump-a/ dump-b/
```

`capture.mjs` throws if a scene renders on any adapter but SwiftShader.
It prints each scene's number of distinct hashes.

On Linux, Chromium sometimes still holds the temporary profile when the
browser closes. The launcher then warns `left /tmp/div-fidelity-…:
ENOTEMPTY` and leaves the folder. That doesn't affect the results.
