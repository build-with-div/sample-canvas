# Canvas in Div: design

Status: Phase 1 investigation, revised 13 November 2026 for the
"port the packages, implement the platform" principle. Nothing here is
built in Div. All Div source references are to build-with-div/div at
`565db46a` (`crates/...:line`). Probe results come from `docs/probes` in
this repo; each number comes with its command and conditions.

**Platform decisions.** On 13 November 2026 the user accepted the platform
design decisions that resolve the consolidated plan's open questions. This
document follows them. The ones that apply to canvas:

- **Decision 1, one rendering design.** Partial redraw is designed once by
  the engine: damage regions first, then layers. Canvas slots (§5.1) are a
  client of that design, not a separate mechanism.
- **Decision 4, API shapes.**
  - `requestAnimationFrame` is per document and returns a typed handle in
    every window.
  - `ResizeObserver` is one API with three boxes.
  - Pointer Events use typed enums and bitflags carrying web values, and
    the mouse's `pointerId` is 1.
- **Decision 5, route A.**
  - `getContext("webgpu")` returns a `GPUCanvasContext` on wgpu.
  - The device is shared with Div, and Div recovers from device loss.
  - Translucent canvases over Div content wait for decision 1's layers.
  - The first step is to verify that headless Chromium 147 exposes
    SwiftShader WebGPU.
- **Decision 8, Canvas 2D.** The engine owns it, as a new `div-canvas`
  crate. It is tested with WPT YAML-generated tests and call-trace replay.
- **Decision 12, `node_ref`.** `node_ref` in `view!` is accepted as
  framework work.

**The principle.**

- The samples are ports of popular npm canvas packages.
- What they lack is reported back, and Div implements the web platform
  features those packages run on, faithfully: Chromium's behavior and
  pixels, measured with Div's comparison harness and WPT.
- Workarounds in samples are temporary scaffolding only. Each is labeled
  `// div-gap:` and logged with the platform feature that will replace it.

**The two routes, as platform features:**

- **Route B** is the HTML `<canvas>` element with `getContext("2d")`
  (`CanvasRenderingContext2D`), to spec, checked with WPT `html/canvas` and
  against the original packages in pinned Chromium 147.
- **Route A** ("give me the swapchain") is the same `<canvas>` element with
  `getContext("webgpu")` (`GPUCanvasContext`). That is the web's own
  surface handoff:
  - The app renders with the WebGPU API, which in Rust is wgpu. wgpu
    implements WebGPU in Firefox.
  - The page, here Div, composites the canvas with everything else.

  WebGL2 (`getContext("webgl2")`) is the older route for the same packages
  and comes later (§6).

## 1. Packages and the platform they run on

The packages were chosen from the parent's list, with ECharts added from
the charts session. Their published code was scanned for the platform APIs
they call ([npm-platform-usage.md](npm-platform-usage.md), with commands and
caveats). The counts are static mentions in one build per package, not
runtime frequency.

### Route B packages (2D context)

| Package | What it is | Platform features it depends on (from the scan and by hand) |
| --- | --- | --- |
| **Konva** 10.9.1 | Scene graph with shapes, layers, drag and drop, events | 2D paths, arcs, transforms (`setTransform`), `save`/`restore`, **shadows (88 mentions)**, `imageSmoothingEnabled`, `drawImage`. **Hit testing:** a hidden canvas per layer is drawn in color keys and read back with **`getImageData`**, using `willReadFrequently`. Mouse, touch and pointer events, `setPointerCapture`, `requestAnimationFrame`, `devicePixelRatio`. |
| **Fabric.js** 7.4.0 | Object canvas editor (select, transform, serialize) | **Two stacked canvases** (`lowerCanvasEl`, `upperCanvasEl`). Transforms (66 mentions), `save`/`restore` (73), fill and stroke styles, gradients, patterns, shadows, `globalCompositeOperation`. Per-pixel hit testing through `getImageData`. `toDataURL`. Mouse and touch events, `dblclick`, `contextmenu`, `offsetX`/`offsetY`. |
| **Chart.js** 4.6.1 | Charts | Paths, arcs, **`setLineDash` (15)**, **text** (`font`, `textAlign`/`textBaseline` 29, `fillText`, `measureText`), `clip`, `devicePixelRatio`, `ResizeObserver` (responsive charts), mouse events, `requestAnimationFrame` (animations). |
| **ECharts / zrender** | Charts (charts session) | The same groups as Chart.js, plus stacked canvas layers, `isPointInPath`/`isPointInStroke` and `Path2D` from SVG path data. See the charts session's `charts-canvas-api-requests.md`. |
| **rough.js** 4.6.6 (used by **Excalidraw** 0.18.1) | Hand-drawn shapes; whiteboard | rough.js uses only paths, Béziers, arcs, `stroke`/`fill`, `setLineDash`, `save`/`restore`. Excalidraw adds detached scratch canvases (`createElement("canvas")` 13), text with web fonts (**`FontFace`/`document.fonts`**), `devicePixelRatio` (49), pointer events (43), `getBoundingClientRect`, keyboard, `ResizeObserver`, `toBlob`/`toDataURL`. |
| **PixiJS** 7.4.3 legacy (Canvas renderer) / 8.23.0 | 2D game and graphics engine | Composite operations, `globalAlpha`, patterns, `drawImage` from images, video and canvases, text with `letterSpacing`, `measureText` (v8 measures text on a 2D canvas even when rendering with WebGL or WebGPU), `OffscreenCanvas`, `createImageBitmap`, pointer events. v8 renders through WebGL or **WebGPU** (`getContext("webgpu")`, `navigator.gpu`). |

### Route A packages (GPU context)

| Package | Context | Platform features |
| --- | --- | --- |
| **three.js** 0.187.0 | `WebGLRenderer` (default); **`WebGPURenderer`**, which carries its own WebGL2 fallback (`class WebGLBackend`) | `navigator.gpu.requestAdapter`, `getContext("webgpu")`, `configure({ alphaMode: "premultiplied" \| "opaque" })`, `getCurrentTexture`, `copyExternalImageToTexture` from images and canvases. `requestAnimationFrame`, `devicePixelRatio`, `OffscreenCanvas`, `HTMLVideoElement` textures. |
| **Babylon.js** 9.36.0 | WebGL engine (default) and `WebGPUEngine` | The same WebGPU surface. Also `ctx.filter`, pointer events (31), wheel (20), `contextmenu`, `setPointerCapture`, `toDataURL` (33, screenshots). |
| **deck.gl** 9.4.1 on **luma.gl** 9.4.2 | luma.gl WebGL2 device (default); WebGPU device | `navigator.gpu`, `getContext("webgpu")`, `getCurrentTexture`. deck.gl itself only uses DOM input: pointer, wheel, `dblclick`, `contextmenu`. |

### The ranking this gives

The 2D context features every 2D package uses come first:

1. **Paths and drawing:** `beginPath`/`moveTo`/`lineTo`, Béziers, `arc`,
   `rect`/`roundRect`, `fill`/`stroke`, `fillRect`/`clearRect`.
2. **Line styles and dashes.**
3. **Fill and stroke style** from CSS color strings.
4. **State:** `save`/`restore`.
5. **Transforms and `clip`.**
6. **Element sizing:** the `width`/`height` attributes with
   `devicePixelRatio`.
7. **Animation:** `requestAnimationFrame`.
8. **Input:** pointer and mouse events with element offsets.

Second tier: text and `measureText` (Chart.js, ECharts, Excalidraw), shadows
(Konva, Fabric), `getImageData` (Konva and Fabric hit testing), `drawImage`
and `globalAlpha`/composite operations (PixiJS, Fabric), `ResizeObserver`
(Chart.js, Excalidraw).

Last tier: `ctx.filter`, `OffscreenCanvas`, `createImageBitmap`, conic
gradients (zero mentions in these packages) and WebGL.

## 2. The platform Div must implement

Each row is a web platform feature, named as the web names it. It lists
which packages need it, what Div has at `565db46a`, what is missing, the
owning track, the size (S, M, L, XL) and how fidelity is measured. IDs (W for web platform) are
used in [rfc-canvas.md](rfc-canvas.md) and
[framework-gaps.md](framework-gaps.md).

| ID | Platform feature | Needed by | Div today | Missing | Owner | Size | Fidelity measure |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **W1** | `HTMLCanvasElement`: `width`/`height` attributes as the bitmap size, default 300×150, CSS scaling of the bitmap, `getContext` | all | Tag, accessibility role and replaced box exist; it paints nothing; with no CSS size it is **300×0** (probe P1) | Bitmap size from attributes; intrinsic size; showing the bitmap | engine | S | WPT `html/canvas/element/canvas-host` and size reftests; census pages with canvases |
| **W2** | `CanvasRenderingContext2D` core: paths, `fill`/`stroke` with fill rule, rects, line styles, `setLineDash`, `save`/`restore`, transforms, `clip`, `globalAlpha`, `globalCompositeOperation` | all 2D | none | everything: a port of Blink's `modules/canvas/canvas2d` onto Skia with Chromium's paint flags | engine | L (split below) | WPT M1 (§7); package traces |
| **W3** | `fillStyle`/`strokeStyle`: CSS color strings, `CanvasGradient` (linear, radial, conic), `CanvasPattern` | all 2D | CSS color parser and gradient shaders in div-style and div-paint | binding to the context; Blink's gradient and pattern semantics | engine | M | WPT `fill-and-stroke-styles` |
| **W4** | Shadows (`shadowBlur`/`shadowColor`/`shadowOffsetX`/`Y`) | Konva, Fabric, PixiJS | box-shadow blur code (`painter.rs:19000-19053`) | the canvas shadow model (blur sigma, transform independence) | engine | S/M | WPT `shadows` |
| **W5** | Text: `font`, `textAlign`, `textBaseline`, `direction`, `letterSpacing`, `fillText`/`strokeText`, `measureText` → `TextMetrics` | Chart.js, ECharts, Konva, Excalidraw, PixiJS | div-text shaping, the `font` shorthand parser | context binding; Blink's text metrics | engine | M | WPT `text`; Chart.js and ECharts traces |
| **W6** | `drawImage` from images and canvases, `imageSmoothingEnabled`/`Quality` | Konva, Fabric, PixiJS, Excalidraw | image resources and decoding | context binding; canvas as a source | engine | M | WPT `drawing-images-to-the-canvas` |
| **W7** | `getImageData`/`putImageData`/`createImageData`, `willReadFrequently` | Konva and Fabric hit testing, PixiJS | none | the readback path; unpremultiply exactly as Chromium does | engine | S | WPT `pixel-manipulation` |
| **W8** | `Path2D` (including from SVG path data), `isPointInPath`/`isPointInStroke`, `roundRect` | ECharts, Chart.js, Konva | none | port of Blink's `path_2d` and the hit-test functions | engine | M | WPT `path-objects` |
| **W9** | `toDataURL`/`toBlob` (PNG) | Fabric, Konva, Babylon, Excalidraw | PNG encoding exists for snapshots (`testing` feature) | the canvas API | framework | S | byte comparison of decoded PNGs with Chromium |
| **W10** | Detached canvases (`document.createElement("canvas")` not in the tree) and **stacked canvases** in one container | Fabric, Excalidraw, Babylon, ECharts | elements can exist detached | a working context on a detached canvas; cheap stacking (§5.1) | engine | S | package traces |
| **W11** | `requestAnimationFrame` / `cancelAnimationFrame` | all | none (probe `frame_loop` uses `PlatformEvent::Presented`) | the callback list in `begin_frame`; headless frame advance | framework | S/M | ordering tests against the HTML event-loop spec |
| **W12** | `devicePixelRatio` and its change notification (`matchMedia("(resolution: …)")`) | all | scale only inside `ViewportMetrics`; no public getter | getter plus change event | framework | S | — |
| **W13** | `ResizeObserver`, including `devicePixelContentBoxSize` | Chart.js, Excalidraw, PixiJS 8, Babylon | none | the observer: engine computes it after layout, framework delivers it | engine + framework | M | WPT `resize-observer` (subset) |
| **W14** | Pointer Events: `pointer*` from the mouse, `pointerType`, `pressure`, `buttons`, `setPointerCapture` semantics, `offsetX`/`offsetY`; also `wheel`, `dblclick`, `contextmenu` | all | `mouse*` only from a mouse; viewport coordinates only; capture works (probe P6) | listed | framework + engine | M | WPT `pointerevents` (subset); probe P6 |
| **W15** | `FontFace` / `document.fonts` (load and `ready`) | Excalidraw, PixiJS | `register_fonts` (app fonts) | the FontFace API and its load state | framework | S/M | — |
| **W16** | Canvas fallback content: children not rendered, exposed to accessibility and focus | HTML requirement; ECharts `aria` | `Role::Canvas` | the fallback semantics | engine | S | accessibility tree snapshot against Chromium |
| **W17** | `navigator.gpu`, `GPUCanvasContext` (`configure`, `getCurrentTexture`, `alphaMode`, `unconfigure`) on wgpu | three.js `WebGPURenderer`, Babylon `WebGPUEngine`, luma.gl, PixiJS 8 | none; presentation is a CPU frame uploaded to one GL texture | a wgpu presenter that composites canvas textures with Div's frame (§5.2) | framework (platform) + engine (scene) | L | WebGPU CTS subset for the context; composition compared with Chromium (§7.3) |
| **W18** | `copyExternalImageToTexture` from images and canvases | three.js, Babylon | none | the copy, after W17 | framework | S | WebGPU CTS |
| **W19** | `OffscreenCanvas` and `createImageBitmap` | PixiJS, three.js, Babylon | none | both APIs; off-thread drawing | engine + framework | M | WPT `offscreen` |
| **W20** | `ctx.filter`, `beginLayer`/`endLayer`, `reset`, color spaces | PixiJS 8, Babylon, Excalidraw (few) | blur and grayscale filters for CSS | listed | engine | L | WPT `filters`, `layers` |
| **W21** | WebGL2 (`getContext("webgl2")`) | three.js and Babylon defaults, deck.gl | none | a GLES context (Chromium implements it with ANGLE) | framework | XL | WebGL CTS (later) |

Rows W2–W8 together are the `div-canvas` engine crate: a port of Blink's
`third_party/blink/renderer/modules/canvas/canvas2d`, not a new design.
**Plain Skia defaults do not match Chromium.** The charts session measured
Skia's default path rendering against Chromium's SVG output: up to 1/255
apart at scale 1, and up to 24/255 at scale 2. The port must carry Chromium's
canvas paint flags and path construction, as Div already does for CSS
painting.

## 3. What Div has today

### Rendering and presentation

- **One picture per document.** Each visual change re-records the whole
  document into one Skia picture (`SkPicture`).
  - `Engine::update_once` lays out if needed, then calls `record_fragment`
    (`div-engine/src/lib.rs:2562-2680`).
  - The damage is always the full viewport (`lib.rs:2686-2691`).
  - The dirty classes are layout, paint, compositing, accessibility and
    hit_test (`lib.rs:310`). Any visual class re-records everything; no
    element is repainted on its own.
- **CPU raster of the whole frame.** `SoftwareCompositor::render` rasterizes
  the picture in 256 px tiles on the CPU (`div-compositor/src/lib.rs:333`,
  `div-paint/src/render.rs:645`). It reads the full frame back into an RGBA
  `Vec<u8>` (`lib.rs:422-457`). A frame is reused only when the generation
  is identical.
- **Two presenters.** Both are in `div-platform/src/desktop.rs`. Either way,
  every frame is a full-window CPU raster followed by a full-window copy.
  - *OpenGL* (glutin: WGL on Windows, EGL/GLX on Linux). It uploads the
    whole RGBA frame with `glTexSubImage2D`, draws one quad and swaps with
    swap interval 1 (`desktop.rs:232, 251-321`). It clears to opaque white,
    so the frame has no alpha.
  - *softbuffer*. It converts every pixel to `0RGB` in a Rust loop
    (`desktop.rs:1726-1755`), with no vsync control.
  - `Auto` tries OpenGL and falls back to softbuffer (`desktop.rs:509-570`).
- **The GPU path is a diagnostic, not a presenter.** `GaneshGlCompositor` is
  an offscreen EGL pbuffer compositor with readback (`ganesh_gl.rs:18-230`).
  It is feature-gated and unpromoted (Div's `docs/alpha/plan.md`: "408/640
  focused exact").
- **Layers are metadata only.** `RetainedScenePlan` and the layered,
  source-layer and surface programs exist (`retained_scene.rs:1-3`,
  `div-paint/src/layered_program.rs:1-3`). They are explicitly diagnostic:
  effects and scroll offsets stay baked into the canonical picture. The only
  separately recorded content is the viewport scrolling layer
  (`div-paint/src/content_layer.rs:10`).
- **The compositor contract forbids callbacks.** `SceneSnapshot` is
  immutable and contains no application callbacks
  (`div-compositor/src/lib.rs:63-76`; Div's `docs/v02/compositor.md`).
  `SceneMailbox` already exists as a one-slot queue for a future render
  thread.

### The `<canvas>` element

- **Known to the DOM, layout and accessibility.**
  - `ElementTag::Canvas` (`div-dom/src/tree.rs:160-180`); `view!` accepts
    `<canvas>` (`divui/src/element.rs:1229`).
  - It lays out as a 300×150 replaced box (`div-layout/src/intrinsic_sizing.rs:3465`)
    and gets the UA `overflow: clip` (`tree.rs:281-296`).
  - Accessibility gives it `Role::Canvas` (`div-engine/src/accessibility.rs:889-896`).
  - It paints nothing: `ReplacedResourceKind::TransparentCanvas` returns
    early in `paint_replaced_content` (`div-paint/src/painter.rs:6331`).
- **Existing pixel path.** Replaced content draws an `SkImage` with
  object-fit, clip and device-pixel snapping (`painter.rs:6311-6530`). Video
  frames already go through an "external texture quad" snap
  (`painter.rs:6398-6416`). An element can be given pixels through
  `Document::register_image_resource` with the undocumented
  `image/x-openui-rgba8` MIME type (`div-paint/src/image_resource.rs:336-372`).
  Three properties of that path matter:
  - Decoded images are cached by the `sha256` string alone, in thread-local
    maps (`image_resource.rs:14-20, 179`).
  - The registry is append-only (`div-dom/src/tree.rs:1042-1067`).
  - `set_image_resource` marks the element **intrinsic**-dirty, so every
    update relayouts (`div-engine/src/lib.rs:2175-2201`).

### Reusable paint code

A 2D context can reuse these:

- **Gradients:** linear, conic and radial shaders (`painter.rs:16477-17255`).
- **Shadows:** the box-shadow blur sigma and mask filter
  (`painter.rs:19000-19053`).
- **Dashes:** `PathEffect::dash` in borders (`painter.rs:22471`).
- **Masks:** DstIn/DstOut blending (`painter.rs:2144`).
- **Text:** shaping and blob drawing in div-text:
  - `TextShaper::shape` (`div-text/src/shaper.rs:506`)
  - `ShapeResult::to_text_blob` (`shape_result.rs:464`)
  - `paint_text` (`div-paint/src/text_painter.rs:49`)
  - the `font` shorthand parser (`div-style/src/property.rs:1239`)

SVG strokes and `mix-blend-mode` do not exist (`image_resource.rs:2260-2624`).

### Public `divui` API that matters here

| Need | Today |
| --- | --- |
| Raw Skia | Not re-exported. An app can depend on `skia-safe` 0.94 itself; with Div's patch table it links the same rust-skia rev (probe P2). |
| Window handle | `NativeWindow::win32()` / `x11()` exist in div-platform (`lib.rs:169-210`), but divui only uses them for dialogs (`divui/src/dialog.rs:140-155`). Not public. |
| Scale factor, window size | No getter and no resize callback. `PlatformEvent::Resized(ViewportMetrics)` reaches `App::on_platform_event` only. |
| Per-frame callback | None. The nearest is `PlatformEvent::Presented` via `App::on_platform_event` (`divui/src/app.rs:198`), on the single-window `App` path only. Frames are on demand: `wants_frame = is_animating() \|\| needs_redraw()` (`desktop.rs:827`). |
| Cross-thread wake | `LoopWaker` exists in div-platform (`lib.rs:148`); divui uses it only for its task runtime. |
| Pointer input | `Event` has `mouse_x`/`mouse_y` in viewport coordinates only (`divui/src/events.rs:175-191`). There are no offset coordinates, `buttons`, pressure or pointer type. A real mouse produces only `mouse*` events, because pointer id 0 maps to `mouse*` names (`document.rs:1068`). `set_pointer_capture` exists (`element.rs:1122`). There are no `contextmenu`, `dblclick` or `resize` events. |
| Hit testing | `Engine::hit_test` computes element-local coordinates internally (`div-engine/src/lib.rs:2731-2745`, `HitEntry.world_to_local`), but returns only the node. |
| Accessibility | `set_accessibility_role/label/description/value` (`divui/src/element.rs:750-770`). There is no API for virtual children. |
| Headless | `HeadlessApp::render_at`, `Document::render_to_bitmap`; `testing::TestApp` has no frame advance and no pointer helpers. |

## 4. Probes

`docs/probes` builds against Div `565db46a` and uses only the public API,
plus the app's own `skia-safe`. To run the probes on Windows:

```
. tools\win-build.ps1
cd docs\probes
cargo test --release -- --nocapture --test-threads 1
```

Results, with machine and load conditions, are in
[framework-gaps.md](framework-gaps.md#probe-results). The machine was at
100% CPU from concurrent builds, so the timings are pessimistic.

| # | Question | Answer |
| --- | --- | --- |
| P1 | Is `<canvas>` usable in `view!`? | Yes, and it paints nothing. But with no CSS size it is **300×0** (as a block, 100%×0), not HTML's 300×150. |
| P2 | Can an app draw with Skia and show the result in an element? | Yes, through `image/x-openui-rgba8` (the workaround below). |
| P3 | Can a pushed frame reuse its cache key? | No. Div shows the old pixels; each frame needs a fresh key, and memory grows without bound. |
| P4 | Is a 2× backing store at device scale 2 mapped 1:1? | Yes. One-device-pixel columns survive exactly. |
| P5 | Does Div content stack over a canvas, and does a scroller clip it? | Yes, because it is ordinary replaced content. |
| P6 | Can an app get element-local pointer coordinates? | Only as viewport minus bounding rect, which is wrong under transforms. Pointer capture does deliver moves outside the element. A mouse produces no `pointer*` events. |
| P7 | What does one pushed canvas frame cost headless? | 640×480 canvas, 1024×768 viewport: about 2 ms app draw and handoff, then **25–29 ms** Div render. A render with no change costs 8–9 ms. |
| `frame_loop` | Can an app animate natively? | Yes, through `Presented`. An 800×600 window reaches **44 fps** on OpenGL and 41 fps on softbuffer, on a 239 Hz display, so the limit is Div's frame, not vsync. |

**About the probe workaround.** `SurfaceBridge` in `docs/probes/src/lib.rs`
draws with the app's own `skia-safe`, reads the pixels back, wraps them in
Div's undocumented `image/x-openui-rgba8` transport and sets the result as
the element's image. Each canvas frame costs:

- a relayout;
- a whole-document re-record;
- a full-window raster and presentation copy;
- a permanent leak of the frame's bytes.

**This is a measurement tool, not a design.** It drew the numbers above and
shows what W1, W2 and §5.1 must fix. The samples do not use it to stand in
for the 2D context (§9).

## 5. Architecture inside Div

### 5.1 One element, two contexts, one compositing mechanism

```
 <canvas width height>  ── HTMLCanvasElement (layout, style, events, a11y, hit test: unchanged)
        │ getContext("2d")                     │ getContext("webgpu")
        ▼                                      ▼
 CanvasRenderingContext2D                 GPUCanvasContext
 (div-canvas: Blink canvas2d on           (wgpu: configure(device, format, alphaMode),
  a CPU raster SkSurface, like             getCurrentTexture() → GPUTexture)
  Chromium's unaccelerated canvas)
        │ commit at frame end                  │ texture presented at frame end
        ▼                                      ▼
            canvas SLOT: versioned contents outside the document lifecycle
            (CPU image | GPU texture), geometry recorded by paint
                                  │
                                  ▼
 SceneSnapshot: document picture + slot geometry (device quad, clip, opacity, occlusion)
                                  │
             ┌────────────────────┴──────────────────────┐
             ▼                                           ▼
 CPU compositor: re-raster only the tiles      wgpu presenter: Div frame as one texture
 under a changed slot (damage = slot rect)     with alpha + canvas textures as quads
```

- **A canvas content change is not a document mutation.** It neither
  relayouts nor re-records. It bumps the slot version and requests a frame
  whose damage is the canvas's device rect. Chromium works the same way:
  canvas and WebGPU contents reach the compositor as their own layers.
- **Geometry still goes through the document.** Moving, resizing,
  scrolling, clipping or overlapping the canvas re-records as today, with
  the slot at its new place. Slot geometry is part of the immutable scene,
  so composition stays consistent: a frame is the pair (scene generation,
  slot versions).
- **Stacked canvases (W10) are slots in z-order.** An overlay redraw, such
  as a crosshair over a 100k-point series, touches only its own slot.
- **Slots sit on the engine's partial-redraw design** (decision 1).
  - A slot version change is one more source of damage regions. The
    engine's damage tracking and partial raster (its Tier 0 item 0.7)
    turn those regions into a partial frame.
  - The GPU texture quads in §5.2 are layers of that same design.
  - Div gets one scene model, not one for canvases and another for
    animation.
- **The 2D context keeps Chromium's model.** Draw calls go to the bitmap
  immediately. The element shows the committed bitmap at the next frame.
  `getImageData` sees every draw so far.

### 5.2 Route A: `getContext("webgpu")`

**Decided** (decision 5). Div owns presentation and composites canvas
textures, as a browser does. The app gets its GPU objects the web way:

```rust
let adapter = div::gpu().request_adapter(&Default::default()).await?; // navigator.gpu
let (device, queue) = adapter.request_device(&Default::default()).await?;
let context = canvas.get_context_webgpu()?;                           // GPUCanvasContext
context.configure(&GpuCanvasConfiguration {
    device: device.clone(),
    format: div::gpu().preferred_canvas_format(),
    alpha_mode: GpuCanvasAlphaMode::Premultiplied,
})?;
// per frame, from document.request_animation_frame(...), which returns a typed handle:
let texture = context.get_current_texture()?;                         // wgpu::Texture
/* render into texture with the WebGPU API (wgpu) */
```

`div::gpu()` hands out adapters and devices from Div's own wgpu instance,
so canvas textures and Div's compositor share one GPU device. The
compositor samples the canvas texture directly, with no cross-API interop.
The earlier draft's D3D11 shared handles and dma-buf are not needed, and the
same code runs on D3D12, Vulkan and (later) Metal.

This needs a presenter change (W17):

- **Div's own content stays CPU-rastered** (the Chromium-exact path). It is
  uploaded as one texture **with alpha**.
- **The presenter composites on wgpu.** It draws the canvas textures with
  the slot geometry: clip, transform, opacity.
- **Z-order uses underlay with a hole punch.** Div's frame is transparent
  over the canvas, and the canvas texture is drawn under it. Content above
  the canvas, such as popovers, stays above. Translucent canvas pixels over
  Div content *below* the canvas need a third layer. That waits for the
  layers of the engine's one rendering design (decision 1; risk R3).
- **Device loss is recovered** (decision 5).
  - Div recreates its device and keeps presenting its own content.
  - The canvas context reports loss to the app, as WebGPU's
    `device.lost` does, and the app recreates its resources.
- **The software presenter cannot sample GPU textures.** On softbuffer, a
  WebGPU canvas reads its texture back each frame. This is slow but
  correct, and it is reported as such.

**Options considered for route A** (the "surface handoff" in general):

| | 1: Skia canvas callback in Div's paint | 2: GPU texture composited by Div (recommended, as `GPUCanvasContext`) | 3: native child surface (not proposed) |
| --- | --- | --- | --- |
| **What Div's present path needs** | A callback in the scene, which Div's compositor contract forbids (`div-compositor/src/lib.rs:63-76`). | A wgpu presenter that keeps Div's frame as a texture **with alpha**, plus a quad per canvas texture; the scene carries slot quads and clips. With Div's device shared (§5.2), no cross-API interop. If devices were separate: D3D11 NT handles or `WGL_NV_DX_interop2` on Windows, dma-buf on Linux, which are driver-dependent. Software presentation reads the texture back. | `with_parent_window` (winit 0.30) on Windows and X11. Div moves and sizes the child at its element's device rect each frame. Wayland needs a `wl_subsurface`, which winit does not expose. |
| **Latency and vsync** | Bound to Div's paint pass: each canvas frame re-records the document. | App render plus one composite. The app can render on its own thread and request frames. Vsync comes from Div's swap (interval 1 on GL). | The app's own swapchain and vsync; lowest latency. Div and child present independently, so the child lags or leads during scroll and resize. |
| **Transparency and overlap** | Full: it is part of Div's picture. | **Underlay and hole punch.** Div's frame is transparent over the slot, and the presenter draws the texture *under* the Div frame. Div content above the canvas (popovers, menus) composites correctly. Translucent canvas pixels over Div content *below* the canvas need a third layer (below / slot / above). That waits for real layer partitioning; until then, slots over non-opaque content are flagged. Chromium's viz uses the same underlay strategy for overlays. | None. Nothing Div draws can cover the child, so popovers over the canvas must be separate top-level windows. Only rectangular clipping (window regions are a hack). No alpha. |
| **Input routing** | Div, as for any element. | Div, as for any element. | The OS sends input to the child window. Div must forward it, or the app handles it, which breaks focus, capture and keyboard shortcuts. |
| **Accessibility** | Element. | Element. | A separate native accessibility subtree; Div's AccessKit tree cannot contain it. |
| **Resize flicker** | None: one frame. | None: the old contents stretch until the new size is drawn (§5.3). A frame can wait briefly for the app's new-size texture, like Chromium's surface synchronization. | Classic child-window flicker and gaps during live resize; needs `WM_SIZE` synchronization and DWM tricks. |
| **Windows / X11 / Wayland** | Same everywhere. | One wgpu code path (D3D12 or Vulkan on Windows, Vulkan on Linux); see risk R2. | Workable on Windows and X11. Wayland subsurfaces work in sync mode and can sit *below* the parent with a hole punch, but need raw Wayland code outside winit. |
| **Limits of CPU-only raster today** | Works on CPU, but every canvas frame costs a full re-record and full raster. | Needs a GPU presenter that composites. Today's GL presenter only uploads a CPU frame, so the presenter must change, though Div's own content can stay CPU-rastered. | Independent of Div's raster: it works today apart from the missing window handle. |

**Option 2 is recommended,** in the form above: the web's `GPUCanvasContext`
on wgpu, with Div's device shared.

- Option 1, a Skia canvas callback inside Div's paint pass, has no web
  equivalent. It also breaks Div's compositor contract: no callbacks, and
  the scene must cross threads.
- Option 3, a native child surface, has no web equivalent and cannot be
  overlapped by Div content. It is not proposed.

### 5.3 Shared machinery

- **`requestAnimationFrame` (W11).** Callbacks run in
  `Document::begin_frame`: after pending events, before `advance_time` and
  `update` (`divui/src/document.rs:1001-1005`). This is HTML's order.
  - A pending callback keeps the window producing frames.
  - The callback receives the frame time that the platform passes to
    `render(time_ms)`.
  - Headless tests advance frames deterministically.
- **`ResizeObserver` (W13).** The engine computes observations after
  layout, including `devicePixelContentBoxSize`; the framework delivers
  them before paint, as HTML does. Packages size their canvas bitmaps from
  it, as Chart.js and PixiJS 8 do. Until the new bitmap is drawn, the old
  one stretches, as in Chromium.
- **Pointer Events (W14).** `offsetX`/`offsetY` come from the hit test's
  `world_to_local` (`div-engine/src/lib.rs:2731-2745`), so they are correct
  under transforms. Mouse input produces `pointer*` events first, then
  compatibility `mouse*` events, as Chromium does.
- **Accessibility (W16).** `<canvas>` children are fallback content: not
  rendered, but in the accessibility tree and focus order, as HTML defines.

## 6. Owners and sizes

The engine and framework tracks own everything. The components track has
no required work. Suggested order, smallest useful first:

| Step | Platform features | Track | Size | Unblocks |
| --- | --- | --- | --- | --- |
| 1 | W1 canvas element sizing and bitmap display; W11 `requestAnimationFrame`; W12 `devicePixelRatio` | engine (W1), framework (W11, W12) | S + S/M + S | Every sample can size a canvas and animate |
| 2 | W2 core context (Blink port, first slice: paths, fill, stroke, line styles, dashes, state, transforms, clip, alpha, composite) and W3 styles | engine | L (M + M) | rough.js, the drawing core of Konva and Fabric, Chart.js shapes; WPT M1 core (431 tests) |
| 3 | §5.1 canvas slots and canvas-only frames, on the engine's damage and partial raster (Tier 0 item 0.7, decision 1) | engine (scene, CPU compositor) | M/L | Canvas animation without re-rendering the window (probe P7: 25–29 ms per frame today) |
| 4 | W14 Pointer Events; W13 `ResizeObserver` | framework + engine | M + M | Konva and Fabric interaction; responsive Chart.js and ECharts |
| 5 | W5 text, W7 readback, W4 shadows, W8 `Path2D` and hit tests, W6 `drawImage` | engine | M + S + S/M + M + M | Chart.js, ECharts and Excalidraw text; Konva hit canvas and shadows |
| 6 | W9 export, W10 detached and stacked canvases, W15 `FontFace`, W16 fallback content | framework + engine | S each (W15 S/M) | Export, Excalidraw fonts, accessibility |
| 7 | First, verify that headless Chromium 147 exposes SwiftShader WebGPU (the oracle). Then W17 `GPUCanvasContext` and the wgpu presenter, with a shared device and device-loss recovery; W18 `copyExternalImageToTexture` | framework (platform) + engine (scene) | L + S | three.js `WebGPURenderer`, Babylon `WebGPUEngine`, luma.gl and deck.gl |
| 8 | W19 `OffscreenCanvas` and `createImageBitmap`; W20 filters, layers, reset, color | engine + framework | M + L | PixiJS 8 and Babylon extras |
| later | W21 WebGL2 | framework | XL | WebGL-only code paths |

## 7. Fidelity plan

Three measurements, all against pinned Chromium 147 through Div's harness.
Exactness is the gate wherever the platform defines pixels.

### 7.1 WPT `html/canvas` (the platform)

**Source.** WPT `html/canvas`, as imported in the Chromium 147 tree. The
test list comes from commit `09d377d9` (the local tree listing in Div's
`chromium-wpt` checkout); the YAML definitions were fetched from GitHub at
that commit.

- 4,448 files in total.
- 1,350 `element/` test pages, excluding references. Of these, 1,157 are
  generated from 19 YAML files in `tools/yaml/`, and 193 are hand-written
  (126 of them in `manual/`).

The census is reproducible with
`tools/wpt_canvas_census.py <yaml dir> <tree listing>`; see
[wpt-canvas-census.md](wpt-canvas-census.md).

**Classification** of the 1,157 generated element tests, by the first
blocker each needs:

| Bucket | Tests | Platform rows |
| --- | ---: | --- |
| Core: paths, fill, stroke, gradients, transforms, clip | 431 | W2, W3 |
| Shadows | 50 | W4 |
| Compositing / `globalAlpha` | 16 | W2 |
| Path2D, DOMMatrix, point-in-path | 73 | W8 |
| Pixel readback | 37 | W7 |
| **M1 total** | **607** | |
| Images, `ImageBitmap` and video sources | 212 | W6, W19 |
| Text | 90 | W5 |
| **M2 total** | **302** | |
| JS semantics: exceptions, types, IDL | 125 | Ported where a typed Rust API keeps the behavior observable |
| Async and timers | 37 | W11 |
| Filters, layers, color spaces, reset and host | 86 | W20, W1 |

This is a regex classification of each test's code. It is an estimate
rather than a measurement, and each bucket shifts by a few percent with the
rules.

**How each test runs.** Div already parses WPT scripts into a straight-line
IR (`tools/wpt/javascript_native_mutation_ir.mjs`). A canvas generator
reuses that approach on the YAML `code:` blocks. Most of them are
`ctx.method(args)` and `ctx.prop = value` with literals and simple loops.
For each test, the generator emits:

1. **A Rust test.** WPT's `@assert pixel` and `_assertPixelApprox` become
   assertions with WPT's own tolerance.
2. **A Chromium comparison.** The canvas pixels are captured from pinned
   Chromium 147 (`--disable-gpu`, so Chromium uses its unaccelerated canvas
   on Skia's CPU raster) and compared exactly, RGBA for RGBA, with Div's.

Both numbers are reported per slice; neither is claimed before it is
measured. Reftests (137 `-expected` pairs in `element/`) also go through the
existing reftest path.

### 7.2 The packages (the ports)

Two complementary methods, using the same input both ways:

1. **Call-trace replay** (proposed by the charts session for Chart.js and
   ECharts).
   - Run the original package page in pinned Chromium with a recording
     proxy around `CanvasRenderingContext2D`, which logs every call and
     property write with its arguments.
   - Replay the trace through Div's context, then diff the canvas pixels
     exactly.
   - This isolates the platform (W2–W8) from the port. A difference here is
     a Div canvas bug, never a porting bug.
   - The traces also measure call frequency, replacing the static ranking
     in §1.
2. **Whole-sample comparison.**
   - The Rust port of a package scene (a Konva stage, a rough.js drawing, a
     Chart.js chart) and the original package page render the same input.
   - Div's harness captures the canvas region in both and diffs exactly.
   - A difference not reproduced by the trace replay is a porting bug in
     the sample.

### 7.3 WebGPU (route A)

- **API behavior.** Run a subset of the WebGPU conformance test suite (CTS)
  for `GPUCanvasContext`: `configure`, `getCurrentTexture`, `alphaMode` and
  resize. wgpu's own conformance work covers the device API underneath.
- **Composition.** Canvases cleared to known colors, with `alphaMode`
  opaque and premultiplied, placed under clips, transforms, scrolling and
  overlapping Div content, at device scale 1 and 2. Compare exactly with
  Chromium, which must run WebGPU on a software adapter (SwiftShader).
  Whether pinned Chromium 147 exposes that headlessly has not been checked.
  Checking it is the first step of W17 (decision 5).
- **Shader rasterization.** Rasterization differs between GPUs, so content
  such as a three.js scene is compared with a stated tolerance, never
  claimed exact.

## 8. Sample apps (ports)

Each sample is a Rust port of a package's core, written against the web API
names as Div exposes them. Each runs the package's own demo input, so §7.2
can compare it with the original.

| Sample | Ports | Route | Needs (in order) |
| --- | --- | --- | --- |
| `rough` | rough.js 4.6.6 (generator and canvas renderer, about 170 KB of JS) plus a small Excalidraw-style board | B | W1, W11, W2, W3; then W14 (drawing), W5, W15 (Excalidraw text) |
| `konva` | Konva's core: stage, layers, shapes, drag and drop, color-key hit canvas, shadows | B | W1, W2, W3, W11, W14, W7 (hit canvas), W4, W10 (layers) |
| `three-webgpu` | A three.js `WebGPURenderer` example scene (orbit viewport) in a Div layout with a toolbar, a popover over the viewport and resizable panes | A | W17, W11, W13, W14 |
| (charts session) | Chart.js and ECharts | B | W2–W5, W8, W11, W13 |

PixiJS and Fabric are not planned as samples: Konva and rough.js already
exercise their platform features. They stay in the ranking.

## 9. Scaffolding policy and what can start now

- **No private reimplementation.** No sample ships its own 2D context or
  compositor in place of W2 or W17.
- **Drawing waits for W1 + W2.** Until they land, the samples build what
  needs no platform: rough.js's geometry generator (pure math), Konva's
  scene graph and event routing (tested headless), and the three.js-style
  wgpu scene code (rendered offscreen in tests).
- **The one allowed scaffold** is `SurfaceBridge`, to show those pixels in
  a window. It is labeled `// div-gap: W1/W2` (or `W17` for wgpu) at each
  use and logged in framework-gaps.md, with the platform feature that
  removes it. It is deleted when that feature lands.

## 10. Risks

- **R1. Canvas animation re-renders the whole window until the slots in
  §5.1 land.** Probe P7 measured 25–29 ms per pushed frame for a 640×480
  canvas, under load. With the nested-flex layout cost (sample-git gap 26),
  any relayout per frame would make it worse; a canvas content change must
  never relayout.
- **R2. Sharing Div's wgpu device with app code** (§5.2, decided in
  decision 5 with device-loss recovery) is simpler than
  cross-API interop, but it departs from browsers, where each page gets its
  own device in a separate GPU process.
  - Device loss or validation errors caused by app code could affect Div's
    compositor.
  - Mitigation: the compositor treats a lost device as a reason to fall
    back to CPU presentation, then recreates the device.
  - Alternative: separate devices with cross-device texture sharing
    through wgpu-hal shared handles, at the cost of per-platform interop.
- **R3. Underlays cannot show translucent canvas pixels over Div content
  below the canvas.** That needs three-layer composition, which waits for
  the layers of the engine's one rendering design (decision 1). Div's
  layered capture is diagnostic today.
  - `alphaMode: "premultiplied"` canvases over non-opaque content are
    flagged in debug builds.
  - 2D canvases do not have this problem: their contents are CPU images
    composited in order.
- **R4. Text fidelity.**
  - Canvas text tests depend on font loading (`CanvasTest.ttf`) and on
    `measureText`'s dozen metrics.
  - Chart.js and ECharts lean on text heavily (charts session).
  - This is the largest fidelity bucket after the core.
- **R5. Typed Rust API versus web semantics.**
  - The API keeps the web names and the web behavior underneath: CSS color
    strings parse with Div's CSS parser, and non-finite arguments are
    ignored, as HTML specifies.
  - Typed values are added on top (a `Color`, typed enums for
    `globalCompositeOperation`), never in place of the web behavior.
  - About 125 WPT tests check IDL-level semantics. Those that stay
    observable through the Rust API are ported.
- **R6. WebGL2 (W21) is XL.** three.js and Babylon still default to
  WebGL2. Their WebGPU renderers cover the modern paths, so WebGPU comes
  first.
- **R7. Threading.** `Engine` is `!Send` (`div-engine/src/lib.rs:325`).
  WebGPU rendering and `OffscreenCanvas` work off the UI thread touch only
  the slot table, never the document.
