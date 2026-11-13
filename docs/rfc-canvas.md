# RFC draft: Canvas (2D context and surface handoff)

Status: draft for the Div tracks, 13 November 2026. From the canvas dogfood
session (build-with-div/sample-canvas). Consumers: sample-canvas,
sample-charts, and any app that draws. Requests go to the engine track
(div-paint, div-engine, div-text, a new `div-canvas` crate) and the
framework track (divui, div-platform). The components track has no
required work. It follows Div's cross-track RFC form (`docs/alpha/rfcs/README.md`).
Background, probes and measurements are in [design.md](design.md). Source
references are to Div `565db46a`.

## Problem

Professional desktop apps draw. Charts, editors' minimaps, paint tools,
node graphs and 3D viewports all need pixels that the DOM does not produce.
Div at `565db46a` has none of this:

- **`<canvas>` paints nothing** (`div-paint/src/painter.rs:6331`). With no
  CSS size it is 300×0, not HTML's 300×150 (probe P1).
- **There is no drawing API, Skia access, frame callback or window
  handle.** No scale factor or resize callback either.
- **The only way to show app pixels is a hidden image MIME type**
  (`image/x-openui-rgba8`). Each update relayouts the element, re-records
  the whole document, re-rasters the whole window, and permanently leaks the
  frame (probes P2, P3 and P7).
- **Input carries viewport coordinates only.** There are no `pointer*`
  events from a real mouse.

## Proposal

### 1. Shared foundation

#### 1a. Animation frames (framework)

```rust
// divui::frame
pub fn request_animation_frame(callback: impl FnOnce(f64) + 'static) -> FrameCallbackId;
pub fn cancel_animation_frame(id: FrameCallbackId);

/// Send handle that asks the window to produce a frame (wraps LoopWaker).
#[derive(Clone)]
pub struct FrameRequester { /* Send + Sync */ }
impl FrameRequester { pub fn request(&self); }
impl Document { pub fn frame_requester(&self) -> FrameRequester; }

// testing
impl TestApp { pub fn advance_frame(&mut self, ms: f64); }
```

- **Order.** Callbacks run in `Document::begin_frame`: after pending
  events, before `advance_time` and `update_all`. A callback registered
  during a frame runs in the next frame, as in HTML.
- **Scheduling.** A pending callback counts as "animating"
  (`PlatformApplication::is_animating`), so the loop keeps producing frames
  only while one is pending.

#### 1b. Element-local pointer data (engine E4 + framework F3)

```rust
impl Event {
    /// Position in the target's border-box coordinates, inverting its full
    /// transform (from the hit test's world_to_local), like MouseEvent.offsetX.
    pub fn offset_x(&self) -> f32;
    pub fn offset_y(&self) -> f32;
    /// Pressed buttons as a bitmask (1 primary, 2 secondary, 4 auxiliary).
    pub fn buttons(&self) -> u16;
    pub fn pointer_type(&self) -> PointerType; // Mouse, Pen, Touch
    pub fn pressure(&self) -> f32;             // 0.5 while a mouse button is down
}
```

- **Native mouse input** dispatches `pointerdown`/`pointermove`/`pointerup`
  before the compatibility `mouse*` events, as Chromium does.
- **Pointer capture** retargets moves outside the element to it.
- **New events:** `contextmenu` and `dblclick`.

#### 1c. Canvas size and DPI (framework F2)

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSize {
    pub css: SizeF,      // content box, CSS px
    pub device: SizeU,   // devicePixelContentBoxSize
    pub scale: f32,      // device scale factor
}
impl CanvasElement {
    /// Called after layout and before paint when `CanvasSize` changes.
    pub fn on_resize(&self, f: impl Fn(CanvasSize) + 'static) -> Result<(), Error>;
}
```

#### 1d. Surface slots (engine E1, E2)

This is internal and not public API. Each canvas element owns a slot, and
paint records the slot at its content box. A slot's contents, either an
`SkImage` or an external texture, are versioned outside the document
lifecycle.

- **Contents-only updates** request a composite-only frame, with damage
  equal to the slot's device rect.
- **E1 (first step).** Show an in-memory `SkImage` in a replaced element
  with `InvalidationClass::Paint`, not `Intrinsic`, and with no resource
  registry entry.
- **E2.** Slot geometry in `SceneSnapshot`, the versioned slot table, tile
  reuse in `SoftwareCompositor`.

### 2. Route B: 2D context (engine E3 in `div-canvas`, framework F2 in `divui::canvas`)

```rust
pub struct CanvasElement { /* wraps an Element with tag canvas */ }
impl CanvasElement {
    pub fn create(document: &Document) -> Result<Self, Error>;
    pub fn from_element(element: Element) -> Option<Self>;
    pub fn element(&self) -> &Element;
    /// Bitmap size, like the width/height attributes; ignored when the
    /// resolution is `Device`.
    pub fn set_bitmap_size(&self, width: u32, height: u32) -> Result<(), Error>;
    pub fn set_resolution(&self, resolution: CanvasResolution) -> Result<(), Error>;
    pub fn context_2d(&self, options: Context2dOptions) -> Result<Context2dHandle, Error>;
    pub fn surface(&self, options: SurfaceOptions) -> Result<Surface, Error>; // route A
}

pub enum CanvasResolution {
    /// HTML semantics: bitmap = width x height, scaled to the CSS box.
    Bitmap,
    /// Bitmap tracks devicePixelContentBoxSize; the context is pre-scaled so
    /// drawing is in CSS px and crisp at every scale factor.
    Device,
}

pub struct Context2dOptions { pub alpha: bool, pub will_read_frequently: bool }

/// Cloneable, !Send handle bound to the element; `with` borrows the context.
#[derive(Clone)]
pub struct Context2dHandle { /* … */ }
impl Context2dHandle { pub fn with<R>(&self, f: impl FnOnce(&mut Context2d) -> R) -> R; }

/// CanvasRenderingContext2D, typed. Method names follow HTML in snake_case.
pub struct Context2d { /* Skia raster surface + Blink-ported state */ }
impl Context2d {
    // state
    pub fn save(&mut self); pub fn restore(&mut self); pub fn reset(&mut self);
    // transforms
    pub fn translate(&mut self, x: f32, y: f32); pub fn rotate(&mut self, radians: f32);
    pub fn scale(&mut self, x: f32, y: f32);
    pub fn transform(&mut self, m: Matrix2d); pub fn set_transform(&mut self, m: Matrix2d);
    pub fn get_transform(&self) -> Matrix2d; pub fn reset_transform(&mut self);
    // compositing
    pub fn set_global_alpha(&mut self, a: f32);
    pub fn set_global_composite_operation(&mut self, op: CompositeOperation);
    // styles
    pub fn set_fill_style(&mut self, style: impl Into<PaintStyle>);   // Color | Gradient | Pattern
    pub fn set_stroke_style(&mut self, style: impl Into<PaintStyle>);
    pub fn create_linear_gradient(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Gradient;
    pub fn create_radial_gradient(&self, x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Gradient;
    pub fn create_conic_gradient(&self, angle: f32, x: f32, y: f32) -> Gradient;
    pub fn create_pattern(&self, image: &CanvasImageSource, repetition: Repetition) -> Pattern;
    // line styles
    pub fn set_line_width(&mut self, w: f32); pub fn set_line_cap(&mut self, c: LineCap);
    pub fn set_line_join(&mut self, j: LineJoin); pub fn set_miter_limit(&mut self, m: f32);
    pub fn set_line_dash(&mut self, segments: &[f32]); pub fn set_line_dash_offset(&mut self, o: f32);
    // shadows
    pub fn set_shadow_color(&mut self, c: Color); pub fn set_shadow_blur(&mut self, b: f32);
    pub fn set_shadow_offset(&mut self, x: f32, y: f32);
    // rects and paths
    pub fn clear_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn begin_path(&mut self); pub fn close_path(&mut self);
    pub fn move_to(&mut self, x: f32, y: f32); pub fn line_to(&mut self, x: f32, y: f32);
    pub fn quadratic_curve_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32);
    pub fn bezier_curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32);
    pub fn arc(&mut self, x: f32, y: f32, r: f32, start: f32, end: f32, ccw: bool);
    pub fn arc_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, r: f32);
    pub fn ellipse(&mut self, x: f32, y: f32, rx: f32, ry: f32, rot: f32, start: f32, end: f32, ccw: bool);
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radii: &[CornerRadius]);
    pub fn fill(&mut self, rule: FillRule); pub fn stroke(&mut self);
    pub fn clip(&mut self, rule: FillRule);
    pub fn fill_path(&mut self, path: &Path2d, rule: FillRule);
    pub fn stroke_path(&mut self, path: &Path2d); pub fn clip_path(&mut self, path: &Path2d, rule: FillRule);
    pub fn is_point_in_path(&self, x: f32, y: f32, rule: FillRule) -> bool;
    pub fn is_point_in_stroke(&self, x: f32, y: f32) -> bool;
    // images
    pub fn draw_image(&mut self, image: &CanvasImageSource, dx: f32, dy: f32);
    pub fn draw_image_rect(&mut self, image: &CanvasImageSource, src: RectF, dst: RectF);
    pub fn set_image_smoothing(&mut self, enabled: bool, quality: SmoothingQuality);
    // text (div-text: same fonts, shaping and registered app fonts as the DOM)
    pub fn set_font(&mut self, css_font: &str) -> Result<(), FontParseError>;
    pub fn set_text_align(&mut self, a: TextAlign); pub fn set_text_baseline(&mut self, b: TextBaseline);
    pub fn set_direction(&mut self, d: TextDirection);
    pub fn set_letter_spacing(&mut self, px: f32); pub fn set_word_spacing(&mut self, px: f32);
    pub fn fill_text(&mut self, text: &str, x: f32, y: f32, max_width: Option<f32>);
    pub fn stroke_text(&mut self, text: &str, x: f32, y: f32, max_width: Option<f32>);
    pub fn measure_text(&self, text: &str) -> TextMetrics;
    // pixels (unpremultiplied sRGB RGBA8, as ImageData)
    pub fn get_image_data(&self, x: i32, y: i32, w: u32, h: u32) -> ImageData;
    pub fn put_image_data(&mut self, data: &ImageData, dx: i32, dy: i32);
    pub fn create_image_data(&self, w: u32, h: u32) -> ImageData;
}

/// OffscreenCanvas: the same context with no element. Send, so it can draw
/// on a worker thread; show it by transferring an image to an element.
pub struct OffscreenCanvas2d { /* … */ }
impl OffscreenCanvas2d {
    pub fn new(width: u32, height: u32) -> Self;
    pub fn context(&mut self) -> &mut Context2d;
    pub fn transfer_to_image(&mut self) -> CanvasImage; // Send, cheap (copy-on-write)
}
impl CanvasElement { pub fn show_image(&self, image: CanvasImage) -> Result<(), Error>; }

pub enum CanvasImageSource<'a> {
    Image(ImageResourceId), Canvas(&'a CanvasElement), Offscreen(&'a OffscreenCanvas2d),
    CanvasImage(&'a CanvasImage), ImageData(&'a ImageData),
}
```

**Semantics.**

- **Port, not reinvention.** The implementation ports Blink's
  `modules/canvas/canvas2d` onto Skia. That covers
  arc/ellipse construction, dash phase, shadow blur, composite-op mapping,
  image smoothing and text metrics.
- **Non-finite arguments are ignored, as in HTML.** No `Result` on drawing
  calls.
- **Typed values replace strings,** for example `CompositeOperation` and
  `LineCap`. Only `set_font` parses a CSS string, because fonts are CSS.
- **Drawing is immediate** into a CPU raster surface. The element shows a
  committed snapshot at the next frame, and readback sees every draw so far.

**`view!` component** (framework):

```rust
#[component]
pub fn Canvas(
    resolution: Option<CanvasResolution>,
    options: Option<Context2dOptions>,
    /// Effect: re-runs when signals it reads or the canvas size change.
    draw: impl Fn(&mut Context2d, CanvasSize) + 'static,
) -> ViewNode;
```

### 3. Route A: surface handoff (framework F4 + engine E5)

```rust
pub struct SurfaceOptions {
    pub backend: SurfaceBackend, // Cpu | PreferGpu | Gpu
    pub alpha: bool,
    pub buffers: u8,             // 2 or 3
}
pub struct Surface { /* element-bound, !Send */ }
impl Surface {
    pub fn producer(&self) -> SurfaceProducer;  // Send
    pub fn backend(&self) -> ActiveSurfaceBackend; // what was negotiated
}
pub struct SurfaceProducer { /* Send */ }
impl SurfaceProducer {
    /// Blocks until a buffer is free; sized to the latest CanvasSize::device.
    pub fn next_frame(&mut self) -> SurfaceFrame<'_>;
}
pub struct SurfaceFrame<'a> { /* … */ }
impl SurfaceFrame<'_> {
    pub fn size(&self) -> SizeU;
    pub fn target(&mut self) -> SurfaceTarget<'_>;
    /// Hands the buffer to Div and requests a composite-only frame.
    pub fn present(self);
}
#[non_exhaustive]
pub enum SurfaceTarget<'a> {
    /// Premultiplied RGBA8 rows in a Div-owned buffer.
    Cpu { pixels: &'a mut [u8], stride: usize },
    /// Windows: a D3D11 texture shared as an NT handle, with a keyed mutex.
    D3D11Shared { handle: isize, format: SurfaceFormat, key: u64 },
    /// Linux: dma-buf planes plus a sync file to signal on completion.
    DmaBuf { planes: Vec<DmaBufPlane>, modifier: u64, fourcc: u32 },
}
```

**Composition.**

- **Underlay with a hole punch.** Div rasters its frame with the slot
  region transparent and composites it over the external texture. Div
  content painted above the canvas, such as popovers, stays above it.
  Clip, scroll and transform come from the slot geometry in the scene.
- **Software presentation** negotiates `Cpu`.
- **Translucent surfaces over Div content** need three-layer composition.
  That is out of scope here (design.md R3).

**Escape hatch (framework F5, F6).**

```rust
impl WindowHandle {
    pub fn scale_factor(&self) -> f64;
    pub fn inner_size(&self) -> SizeU;
}
impl raw_window_handle::HasWindowHandle for WindowHandle { … }
impl raw_window_handle::HasDisplayHandle for WindowHandle { … }

/// A native child window kept at the element's device rect. Div content
/// cannot draw over it; input goes to it, not to Div.
pub fn native_child_surface(element: &Element) -> Result<NativeChildSurface, Error>;
```

### 4. Raw Skia (optional, behind a feature)

```rust
#[cfg(feature = "unstable-skia")]
impl Context2d { pub fn with_skia_canvas<R>(&mut self, f: impl FnOnce(&skia_safe::Canvas) -> R) -> R; }
```

- **Why.** Apps that already draw with Skia can mix it with the context's
  state.
- **Why behind a feature.** It ties divui's API to skia-safe's version.
- **Not a paint callback.** It runs outside paint, on the context's
  surface.

## Affected tracks

| Track | Work (size) |
| --- | --- |
| Engine | E1 paint-only in-memory replaced image (S); E2 surface slots and composite-only frames (M/L); E3 `div-canvas` Blink port (L: E3a M, E3b M, E3c S/M, E3d M, E3e S, E3f M, E3g L later); E4 local hit coordinates (S); E5 underlay/hole-punch slot geometry (M); E6 `<canvas>` fallback content not painted, exposed to accessibility (S); E7 image resource eviction (S); E8 `<canvas>` default 300×150 intrinsic size (S). |
| Framework | F1 animation frames and `FrameRequester` (S/M); F2 `divui::canvas` element, component and resize (M); F3 pointer events (M); F4 GL presenter alpha frame, composite-only frames, texture import (L); F5 window info and raw-window-handle (S); F6 native child surface (M, Wayland L). |
| Components | None required. sample-charts may propose chart components on top of `Canvas`. |

## Migration

Every step is additive, so `main` stays green and each step has its own
tests. The order:

1. **F1 animation frames, then E1 paint-only replaced image.** These are
   independent and both small.
   - Tests: headless rAF ordering, and no relayout on an image swap (layout
     count from engine stats).
2. **F2 `divui::canvas` on E1.**
   - At this point `Context2d` can be a thin first slice: E3a only.
   - Its fidelity tests start with WPT M1's core bucket, through the
     generator in design.md §7.
3. **E3 slices**, each with its WPT bucket and an exact-match count against
   pinned Chromium. Counts are recorded per slice, never as a full-census
   claim.
4. **E4 + F3 pointer data.**
   - Tests: offset under rotate/scale, and capture retargeting.
5. **E2 surface slots.**
   - Tests: a slot-only change re-rasters only intersecting tiles, and its
     pixels match a full render. This is an exactness gate: slot frames must
     equal the full re-record.
6. **F5 window info.**
7. **F4 + E5 GPU surfaces.**
   - Windows first (D3D11 interop), with capability fallback and a CI smoke
     test on hosted runners in software mode.
   - Linux dma-buf follows.
8. **F6 native child surface.** Only if a sample needs it.

Nothing in this RFC changes existing rendering. A document without a
`<canvas>` that has a context or surface records the same picture as
before.
