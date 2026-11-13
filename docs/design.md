# Canvas in Div: design

Status: Phase 1 investigation, 13 November 2026. Nothing here is built in Div.
All Div source references are to build-with-div/div at `565db46a`
(`crates/...:line`). Probe results come from `docs/probes` in this repo; the
commands and conditions are given with each number.

The user wants both:

- **A. Surface handoff.** Div reserves an element-sized region and hands
  the app a surface. The app renders into it however it likes. Div then
  composites the surface with the rest of the UI: z-order, clipping,
  scrolling, DPI and resize.
- **B. A built-in 2D context**, like `CanvasRenderingContext2D`, on Div's
  own Skia (Chromium's canvas also runs on Skia). Fidelity against pinned
  Chromium 147 can be measured with WPT.

## 1. What Div has today

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

## 2. Probes

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

**The workaround that works today** is `SurfaceBridge` in
`docs/probes/src/lib.rs`. Each frame it:

1. draws into the app's raster `SkSurface`;
2. reads the pixels back as unpremultiplied RGBA;
3. prefixes the 16-byte `OUIR` header;
4. registers the bytes under a fresh key;
5. calls `set_image_resource`.

This is enough to start the samples (section 7). It costs a relayout, a
whole-document re-record, a full-window raster and a full-window
presentation copy per canvas frame. It also leaks every frame's bytes.

## 3. Recommendation

One mechanism serves both routes: a **surface slot**. A canvas, whether it
holds a 2D context or a handed-off surface, is an ordinary replaced element
for layout, style, hit testing, events and accessibility. Only its *pixels*
live outside the document lifecycle:

```
             UI thread (Document, !Send)                        compositor / presenter
  ┌──────────────────────────────────────────┐      ┌─────────────────────────────────────┐
  │ <canvas> element: layout, style, events, │      │ SceneSnapshot                        │
  │ a11y, hit test (unchanged machinery)     │      │  picture (document, as today)        │
  │                                          │      │  slots: [ SlotGeometry {             │
  │ paint records a SLOT at the content box  │ ───► │    id, device quad, clip (rrect),    │
  │ with its clip/transform/opacity          │      │    opacity, occluded_above } ]       │
  └──────────────────────────────────────────┘      │                                      │
  ┌──────────────────────────────────────────┐      │ SlotTable (versioned contents):      │
  │ B: CanvasContext2d draws into a Skia     │ ───► │   Image(SkImage)      CPU            │
  │    raster surface; commit = new version  │      │   Texture(External)   GPU (route A)  │
  │ A: SurfaceProducer (Send) presents a     │ ───► │ A slot-only change = composite-only   │
  │    frame from any thread                 │      │ frame, damage = slot device rect     │
  └──────────────────────────────────────────┘      └─────────────────────────────────────┘
```

- **A slot update is not a document mutation.** It does not dirty layout or
  paint. It bumps the slot version and requests a frame with damage limited
  to the slot's device rect.
- **On the CPU path**, the compositor rasterizes only the tiles under that
  rect. `SoftwareCompositor` already tiles at 256 px, and the scene already
  carries a `physical_damage` list that nobody uses yet.
- **On the GPU path**, the presenter redraws one textured quad over the
  cached Div frame.
- **Geometry changes** still go through the document: moving, resizing,
  scrolling, clipping or overlapping the canvas re-records it as today, with
  the slot at its new place.
- **Pixels and the document stay consistent.** Slot geometry is part of the
  immutable scene. Slot contents are versioned, and a frame is the pair
  (scene generation, slot versions).

Why not the alternatives:

- **A paint callback** (option A1, a Skia canvas handed to app code during
  Div's paint pass) breaks the compositor contract: no callbacks, and the
  scene must be able to cross threads. It would also force a whole-document
  re-record on every canvas frame. Its useful half, "the app draws with
  Skia", is kept: the app draws into the slot's surface outside paint (B,
  and A on the CPU).
- **A child native surface** (option A3) cannot be overlapped by Div
  content, so popovers, menus and tooltips cannot cover a canvas (§4.3).
  It is kept as an opt-in escape hatch, not the primary route.

### 3.1 Shared pieces

1. **Frame scheduling.** Callbacks run inside `Document::begin_frame`:
   after pending events are dispatched, and before `advance_time` and
   `update` (`divui/src/document.rs:1001-1005`). This is Chromium's order:
   input, then rAF, then style/layout/paint.
   - `request_animation_frame` is one-shot, as in HTML. A pending callback
     makes `PlatformApplication::is_animating` true.
   - The callback receives the frame time that the platform passes to
     `render(time_ms)` (`desktop.rs:858`).
   - For route A, a `FrameRequester` is `Send` and wraps `LoopWaker`, so a
     render thread can ask for a composite-only frame.
   - Headless: `TestApp::advance_frame(ms)` runs the callbacks
     deterministically.
2. **Resize and DPI.** The canvas element gets a `CanvasSize { css, device,
   scale }` callback after layout, before paint. This is the equivalent of
   `ResizeObserver` with `devicePixelContentBoxSize`, which Chromium
   supports. The app (or the 2D context's "auto" mode) reallocates its
   backing store at `device` size. Until a frame at the new size arrives,
   the slot draws the old contents scaled into the new box, as Chromium
   does for a canvas whose CSS size changes. There is never a blank frame.
3. **Input.** A canvas is a normal element, so hit testing, capture and
   focus (`tabindex`) need no canvas-specific code. Two things are missing:
   - element-local coordinates (`Event::offset_x/offset_y`, from the
     `HitEntry.world_to_local` matrix that already exists, so they stay
     correct under transforms);
   - full pointer events: `pointer*` for the mouse, `buttons`,
     `pointer_type`, pressure, and `contextmenu` and `dblclick`.

   Hit regions inside the canvas are the app's job. B adds
   `is_point_in_path` and `is_point_in_stroke`.
4. **Accessibility.** The element already has `Role::Canvas` and authored
   label and description. Following HTML, DOM children of a `<canvas>` are
   fallback content: not painted, exposed in the accessibility tree
   (§6, risk R6). Interactive canvases (charts, editors) eventually need
   virtual accessibility children; that is a separate, later RFC.

### 3.2 Route A: surface handoff, by option

| | A1: Skia canvas in Div's paint | A2: GPU interop (recommended end state) | A3: child native surface (escape hatch) |
| --- | --- | --- | --- |
| **What Div's present path needs** | A slot (CPU image). The paint-callback form needs a callback in the scene, which the contract forbids; the slot form needs none. | A GPU presenter that keeps Div's frame as a texture **with alpha**, plus a quad per external texture. The scene carries slot quads and clips. Shared-texture import: on Windows, D3D11 NT handles opened through `WGL_NV_DX_interop2` (or a D3D11/DirectComposition presenter, see risk R2); on Linux, dma-buf through `EGL_EXT_image_dma_buf_import` or GL/Vulkan external memory. Software presentation falls back to readback (slow) or CPU slots. | `with_parent_window` (winit 0.30) on Windows and X11. Div moves and sizes the child at its element's device rect each frame. Wayland needs a `wl_subsurface`, which winit does not expose. |
| **Latency and vsync** | Bound to Div's frame. CPU raster of the slot's tiles plus the presenter copy. | App render plus one composite. The app can render on its own thread and request frames. Vsync comes from Div's swap (interval 1 on GL). | The app's own swapchain and vsync; lowest latency. Div and child present independently, so the child lags or leads during scroll and resize. |
| **Transparency and overlap** | Full: it is part of Div's picture. | **Underlay and hole punch.** Div's frame is transparent over the slot, and the presenter draws the texture *under* the Div frame. Div content above the canvas (popovers, menus) composites correctly. Translucent canvas pixels over Div content *below* the canvas need a third layer (below / slot / above). That waits for real layer partitioning; until then, slots over non-opaque content are flagged. Chromium's viz uses the same underlay strategy for overlays. | None. Nothing Div draws can cover the child, so popovers over the canvas must be separate top-level windows. Only rectangular clipping (window regions are a hack). No alpha. |
| **Input routing** | Div, as for any element. | Div, as for any element. | The OS sends input to the child window. Div must forward it, or the app handles it, which breaks focus, capture and keyboard shortcuts. |
| **Accessibility** | Element. | Element. | A separate native accessibility subtree; Div's AccessKit tree cannot contain it. |
| **Resize flicker** | None: one frame. | None with the fallback scaling in §3.1.2. A frame can wait briefly for the app's new-size texture, like Chromium's surface synchronization. | Classic child-window flicker and gaps during live resize; needs `WM_SIZE` synchronization and DWM tricks. |
| **Windows / X11 / Wayland** | Same everywhere. | Different interop per platform, and drivers vary (risk R2). | Workable on Windows and X11. Wayland subsurfaces work in sync mode and can sit *below* the parent with a hole punch, but need raw Wayland code outside winit. |
| **Limits of CPU-only raster today** | Workable now. It needs no GPU, but every canvas frame costs a full re-record and full raster until slots land. | Needs a GPU presenter that composites. Today's GL presenter only uploads a CPU frame, so the presenter must change, though Div's own content can stay CPU-rastered. | Independent of Div's raster: it works today apart from the missing window handle. |

**Recommendation for A.** Build A2's model in two stages on the slot:

1. **Stage A-CPU (M).** `SurfaceSlot` with CPU contents. The app renders
   with Skia, or anything that can produce pixels, including wgpu with a
   readback. This is the same slot B uses, so it lands once for both
   routes. It runs on today's CPU raster.
2. **Stage A-GPU (L).** External textures composited by the GL presenter
   as underlays with a hole punch. Its prerequisite is the presenter change
   ("Div frame as an alpha texture, composite-only frames"), which also
   removes the full-frame upload for slot-only frames. Div's own content can
   stay CPU-rastered: only the composition step moves to the GPU. This does
   not require promoting Ganesh.

**A3** stays an opt-in `NativeChildSurface` for embedding renderers that
insist on owning a window, such as video players and some engines. It needs
only a public window handle (S) plus geometry sync (M, Windows and X11).
Its overlap and input limits are documented, not solved.

### 3.3 Route B: built-in 2D context

- **Implementation.** Port Blink's
  `third_party/blink/renderer/modules/canvas/canvas2d/` to a new engine
  crate, `div-canvas`, on skia-safe. This is how Div matches Chromium
  elsewhere, by porting Blink. The relevant files:
  - `base_rendering_context_2d.cc`
  - `canvas_rendering_context_2d_state.cc`
  - `canvas_path.cc`
  - `canvas_style.cc`
  - `canvas_gradient.cc`
  - `canvas_pattern.cc`
  - `path_2d.cc`
  - text from `canvas_rendering_context_2d.cc` and `text_metrics.cc`
- **Backing store.** A CPU raster `SkSurface`, matching Chromium's
  unaccelerated canvas. Div's harness runs Chromium with `--disable-gpu`
  (`tools/accountability/run_all_pixel_comparisons.py:766`), and that
  configuration uses an unaccelerated canvas; this has not been verified
  against canvas tests yet.
- **Drawing model.** Draw calls go straight to the surface. A commit at the
  next frame publishes `image_snapshot()` (copy-on-write) as the slot's new
  version. Readback (`get_image_data`) reads the surface directly. Chromium
  records into a paint-op buffer and flushes at the frame. That deferral is
  an optimization, and the pixels are the same.
- **Text.** `fill_text`, `stroke_text` and `measure_text` go through
  div-text, so canvas text uses the same font collection, shaping and
  registered app fonts as Div's DOM text. The `font` property parses with
  Div's own `font` shorthand parser. Text is where most fidelity work will
  be (§6, risk R4).
- **DPI.** As in HTML, the bitmap size (`width`/`height`) is separate from
  the CSS size, and the default is CSS pixels at 1×. A Div convenience,
  `CanvasResolution::Device`, keeps the bitmap at
  `devicePixelContentBoxSize` and pre-scales the transform by the device
  scale. App code then draws in CSS pixels and is crisp at any DPI, without
  the web's manual `devicePixelRatio` dance.
- **OffscreenCanvas.** `OffscreenCanvas2d` is the same context on a surface
  that no element owns. It is `Send`, so it can draw on a worker thread. It
  transfers to an element with `transfer_to_image()` or by setting it as a
  slot's contents.
- **Animation.** `request_animation_frame` from §3.1. A draw outside a frame
  callback is committed at the next frame, as in HTML.

## 4. Proposed `divui` API (sketch)

The full proposal, with owners and migration, is [rfc-canvas.md](rfc-canvas.md).
In brief:

```rust
use div::prelude::*;
use div::canvas::{Canvas, CanvasResolution, Context2d, FillRule, LineCap};

#[component]
fn Sparkline(values: Signal<Vec<f32>>) -> ViewNode {
    view! {
        <Canvas
            resolution=CanvasResolution::Device      // crisp at any scale
            style:width="240px" style:height="48px"
            draw=move |ctx: &mut Context2d, size: CanvasSize| {
                // re-runs when `values` or the size changes (it is an effect)
                ctx.clear_rect(0.0, 0.0, size.css.width, size.css.height);
                ctx.set_stroke_style(Color::from_rgba8(37, 99, 235, 255));
                ctx.set_line_width(1.5);
                ctx.set_line_cap(LineCap::Round);
                ctx.begin_path();
                for (i, v) in values.get().iter().enumerate() {
                    let x = i as f32 * size.css.width / 31.0;
                    let y = size.css.height * (1.0 - v);
                    if i == 0 { ctx.move_to(x, y) } else { ctx.line_to(x, y) }
                }
                ctx.stroke();
            }
        />
    }
}

// Imperative use, for animation: element + context + rAF.
let canvas = CanvasElement::create(&document)?;          // a <canvas> element
let mut ctx = canvas.context_2d(Context2dOptions::default())?;
fn tick(ctx: Context2dHandle, time_ms: f64) {
    ctx.with(|ctx| { /* draw the frame */ });
    request_animation_frame(move |t| tick(ctx, t));       // one-shot, as in HTML
}

// Route A: hand-off. The producer is Send and lives on the app's render thread.
let surface = canvas.surface(SurfaceOptions { backend: SurfaceBackend::PreferGpu })?;
let mut producer = surface.producer();                    // impl Send
std::thread::spawn(move || loop {
    let frame = producer.next_frame();                    // waits for a free buffer
    match frame.target() {
        SurfaceTarget::Cpu(pixels) => draw_cpu(pixels, frame.size()),
        SurfaceTarget::D3D11 { shared_handle, .. } => draw_wgpu_d3d12(shared_handle),
        SurfaceTarget::DmaBuf { planes, .. } => draw_vulkan(planes),
    }
    frame.present();                                      // composite-only frame
});
canvas.on_resize(|size: CanvasSize| { /* device px, scale */ })?;
```

## 5. Who owns what

| # | Piece | Track | Size |
| --- | --- | --- | --- |
| E1 | Canvas content from an in-memory `SkImage` with **paint-only** invalidation, instead of the resource registry (no key, no relayout, no leak) | engine | S |
| E2 | Surface slots: slot geometry in `SceneSnapshot`, versioned slot table, composite-only frames, damage limited to the slot rect, CPU tile reuse | engine | M/L |
| E3 | `div-canvas`: Blink's 2D context ported onto Skia (see breakdown) | engine | L |
| E3a | State stack, paths, fill and stroke, line styles and dashes, transforms, clip, `globalAlpha`, composite operations | engine | M |
| E3b | Gradients, patterns, `drawImage`, image smoothing | engine | M |
| E3c | Shadows | engine | S/M |
| E3d | Text (`fill_text`, `stroke_text`, `measure_text`, font parsing) through div-text | engine | M |
| E3e | `get_image_data`, `put_image_data`, `create_image_data` | engine | S |
| E3f | `Path2D`, `is_point_in_path`/`is_point_in_stroke`, `round_rect` | engine | M |
| E3g | Later: `filter`, layers (`beginLayer`), `reset`, color spaces | engine | L |
| E4 | Element-local hit coordinates out of `HitEntry` | engine | S |
| E5 | Scene support for external-texture underlays: slot quads, "occluded above", hole punch in Div's raster | engine | M |
| E6 | `<canvas>` fallback children: not laid out or painted, exposed to accessibility | engine | S |
| E7 | Image resource lifetime: unregister and cache eviction (today keyed by `sha256`, never freed) | engine | S |
| E8 | `<canvas>` default intrinsic size 300×150 when it has no content (today 300×0, or 100%×0 as a block; probe P1) | engine | S |
| F1 | `request_animation_frame`, a `Send` `FrameRequester`, `TestApp::advance_frame` | framework | S/M |
| F2 | `divui::canvas`: `CanvasElement`, `Canvas` component, `CanvasSize`/resize callback, `Context2d` wrapper, `CanvasResolution` | framework | M |
| F3 | Pointer events: `pointer*` for the mouse, `offset_x`/`offset_y`, `buttons`, `pointer_type`, pressure, `contextmenu`, `dblclick`; native capture | framework (with E4) | M |
| F4 | GL presenter: Div frame as an alpha texture, composite-only frames, external texture import (Windows D3D11 interop, Linux dma-buf), producer/consumer synchronization | framework | L |
| F5 | Public window info: scale factor, size, `raw-window-handle` (`HasWindowHandle`/`HasDisplayHandle`) | framework | S |
| F6 | `NativeChildSurface` escape hatch (Windows, X11; Wayland later) | framework | M (+L for Wayland) |
| C1 | None required. Optional later: themed chart and canvas wrappers, coordinated with sample-charts | components | — |

Suggested order, smallest useful first: **F1, E1+E8, F2 (B on today's
replaced-image path), E3a–E3e, E4+F3, E2, F5**, then **F4+E5** (A-GPU), then
**F6**.

- After E1+F1+F2, the samples stop leaking and stop relayouting.
- After E2, a canvas frame costs its own tiles, not the window.

## 6. Risks

- **R1. Every canvas frame re-renders the window until E2 lands.** It costs
  a whole-document re-record, full raster and full upload. Measured under
  load: 25–29 ms per pushed frame headless, and 44 fps in an 800×600 window
  on a 239 Hz display; the app's own draw and handoff take about 2 ms of
  that. Layout stays out of it once E1
  replaces the intrinsic-dirty path. With the known nested-flex cost
  (sample-git gap 26), a relayout per frame would be severe, which is why E1
  comes first.
- **R2. GPU interop is driver-dependent.**
  - `WGL_NV_DX_interop2` is widely available on desktop Windows drivers,
    but it is an extension, and some virtual GPUs and remote sessions lack
    it.
  - dma-buf import varies by Mesa and NVIDIA version.
  - Mitigation: capability negotiation (`SurfaceBackend::PreferGpu` falls
    back to CPU slots), plus a CI smoke test per platform.
  - Alternative on Windows: present with D3D11 and DirectComposition
    instead of WGL. Chromium does this. It makes shared handles native and
    lets Windows scan out overlays, but it is a second presenter to
    maintain.
- **R3. Underlays cannot show translucent canvas pixels over Div content
  below the canvas.** That needs three-layer composition (below / slot /
  above), which waits for real layer partitioning (Div's layered capture is
  diagnostic today). Until then a translucent external surface draws over a
  solid backdrop color, and Div should flag it in debug builds.
- **R4. Text fidelity.**
  - WPT canvas text tests depend on font loading (`CanvasTest.ttf`) and
    exact metrics.
  - `measureText` returns about 12 metrics; each must come from div-text's
    shaping, as Blink computes them.
  - This is the largest fidelity bucket after core drawing.
- **R5. API shape versus HTML.**
  - HTML's dynamic typing (strings for colors, `NaN` arguments ignored,
    exceptions) does not map to Rust one to one. About 125 WPT tests check
    those semantics.
  - Proposal: typed setters (`Color`, `f32`), and HTML's "ignore non-finite
    arguments" kept as a documented behavior, so porting stays mechanical.
  - Rule: deviate from HTML only where Rust's types make the HTML case
    impossible.
- **R6. Fallback content.** Div may lay out and paint `<canvas>` children
  today. This is not checked yet; E6 makes it explicit.
- **R7. Threading.** `Engine` is `!Send` (`div-engine/src/lib.rs:325`).
  - Route A producers and `OffscreenCanvas2d` must not touch the document;
    the slot table is the only shared state.
  - Div's presenter runs on the event-loop thread
    (`docs/architecture/threading-model.md`). A render thread is not needed
    for this design, but A-GPU's synchronization (keyed mutex or fences)
    must not block that thread.
- **R8. The workaround leaks.** Every `SurfaceBridge` frame adds a
  registered resource and a cache entry that are never freed (P3). Samples
  built before E1/E7 must cap their frame rate or run briefly; they say so
  in their READMEs.

## 7. Fidelity plan for route B

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

| Bucket | Tests | Milestone |
| --- | ---: | --- |
| Core: paths, fill, stroke, gradients, transforms, clip | 431 | M1 (E3a, E3b) |
| Shadows | 50 | M1 (E3c) |
| Compositing / `globalAlpha` | 16 | M1 |
| Path2D, DOMMatrix, point-in-path | 73 | M1 (E3f) |
| Pixel readback | 37 | M1 (E3e) |
| **M1 total** | **607** | |
| Images, `ImageBitmap` and video sources | 212 | M2 (E3b; Div image resources replace `<img>` and `ImageBitmap`) |
| Text | 90 | M2 (E3d) |
| **M2 total** | **302** | |
| JS semantics: exceptions, types, IDL | 125 | Ported as typed-API tests where meaningful; most become compile-time guarantees |
| Async and timers | 37 | Ported with `advance_frame` where they test drawing |
| Filters, layers, color spaces, reset and host | 86 | Deferred (E3g) |

This is a regex classification of each test's code. It is an estimate
rather than a measurement, and each bucket shifts by a few percent with the
rules.

**How each test runs.** Div already parses WPT scripts into a straight-line
IR (`tools/wpt/javascript_native_mutation_ir.mjs`). A canvas generator
reuses that approach on the YAML `code:` blocks. Most of them are
`ctx.method(args)` and `ctx.prop = value` with literals and simple loops.
For each test, the generator emits:

1. **A Rust test** calling `div-canvas`. Each `@assert pixel` and
   `_assertPixelApprox` becomes a pixel assertion with WPT's own tolerance.
   These run in Div CI on Windows and Linux, without Chromium.
2. **A Chromium comparison case.** The canvas pixels are captured from
   pinned Chromium 147 with Div's harness (`--disable-gpu`, CDP device
   scale), then compared exactly, RGBA for RGBA, with the Div output. The
   canvas element is screenshotted alone, so the testharness text is
   excluded. Exactness is the gate, as in Div's census. WPT's
   `@assert pixel` is the weaker floor.

M1 reports both numbers: WPT assertions passing, and exact Chromium matches.
Neither number is claimed before it is measured. Reftests (137 `-expected`
pairs in `element/`) also go through the existing reftest path.

## 8. Sample apps

| Sample | Route | Proves |
| --- | --- | --- |
| `particles` | B | rAF-driven animation: 2D context, 10k particles, `globalAlpha`, additive composite, frame-time overlay. The first Phase 2 sample, on the `SurfaceBridge` workaround plus app-side Skia. |
| `paint` | B | Pointer input: offset coordinates, capture, pressure (where the platform gives it), undo through `get_image_data`/`put_image_data`, brush strokes with round caps and joins, a color picker built in Div over the canvas. |
| `canvas-gallery` | B | A WPT-style visual gallery: gradients, patterns, dashes, shadows, text with `measure_text`. The same scenes are compared with Chromium, so fidelity is visible in the sample. |
| `viewport3d` | A | A wgpu 3D viewport (orbit camera) inside a Div layout, with a Div toolbar, a popover menu over the viewport, resizable split panes and a scrolled inspector. Exercises overlap, clip, resize and DPI. Starts with CPU readback (A-CPU) and moves to shared textures (A-GPU) when F4 lands. |
| `video-embed` (later) | A3 | A native child surface. Documents the overlap and input limits rather than hiding them. |

The charts session (`sample-charts`) may build on B's `Canvas` component.
Its axis-label text and hit-testing needs feed E3d and F3.

## 9. What can be prototyped now, without Div changes

- **B in userland.**
  - A `canvas2d` crate in this repo exposes the proposed `Context2d` API
    over the app's own skia-safe and shows it through `SurfaceBridge`.
  - Text through the app's Skia uses fonts that Div's DOM text does not
    share. This is a known divergence until E3d.
  - It lets `particles`, `paint` and `canvas-gallery` start, and it tests
    the API shape before the engine ports Blink.
  - It is *not* the fidelity implementation.
- **A on the CPU.** Render with wgpu offscreen, read back, then use
  `SurfaceBridge`. This works but is slow; `viewport3d` measures it.
- **Frame loop.** `App::on_platform_event` plus `Presented`, on the
  single-window `App` path only (`frame_loop` probe).
- **Not possible without Div changes:**
  - A-GPU: no GPU compositing, and no window handle.
  - A3: a window handle could be found with `FindWindowW` by title, which is
    a hack.
  - Element-local coordinates under transforms.
  - `pointer*` events from a real mouse.
  - Freeing pushed frames.
