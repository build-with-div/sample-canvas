# RFC draft: the HTML canvas element (2D and WebGPU contexts)

Status: draft for the Div tracks, revised 13 November 2026 for the
"port the packages, implement the platform" principle.

- **From:** the canvas dogfood session (build-with-div/sample-canvas).
- **Consumers:** ports of Konva, rough.js/Excalidraw and three.js in
  sample-canvas; Chart.js and ECharts in sample-charts; any app that draws.
- **Requests to:** the engine track (div-paint, div-engine, div-text, a new
  `div-canvas` crate) and the framework track (divui, div-platform). The
  components track has no required work.
- **Form:** Div's cross-track RFC form (`docs/alpha/rfcs/README.md`).
- **Background:** [design.md](design.md) has the package scan, the platform
  table (W1–W21), probes and measurements. Source references are to Div
  `565db46a`.
- **Decisions:** this draft follows the platform design decisions the user
  accepted on 13 November 2026, which design.md lists. Under decision 4,
  the framework track's own RFC sets the exact shapes for
  `requestAnimationFrame`, `ResizeObserver` and Pointer Events; the
  sketches below follow its directions.

## Problem

The popular canvas packages, listed in design.md §1, run on a small set of
web platform features: the `<canvas>` element, `CanvasRenderingContext2D`,
`GPUCanvasContext`, `requestAnimationFrame`, `ResizeObserver`, Pointer
Events and `devicePixelRatio`. Div at `565db46a` implements none of them
beyond the bare element:

- **`<canvas>` paints nothing** (`div-paint/src/painter.rs:6331`). With no
  CSS size it is 300×0, not HTML's 300×150 (probe P1).
- **No `getContext`, no `requestAnimationFrame`, no `ResizeObserver`, no
  `devicePixelRatio`.**
- **Input is incomplete.** There are no `pointer*` events from a mouse, and
  no `offsetX`/`offsetY`.
- **No GPU composition.**
- **The only way to show app pixels is a hidden image MIME type.** Each
  update relayouts, re-records the whole document, re-rasters the whole
  window and leaks the frame (probes P2, P3, P7).

A port of any of these packages cannot draw.

## Proposal

**Implement the web platform features, with web names and web semantics.**
The Rust API is the WebIDL in snake_case:

- Members keep their web names (`fill_style`, `get_image_data`).
- Behavior follows the spec and Chromium: CSS color strings parse with
  Div's CSS parser, non-finite arguments are ignored, and state defaults are
  the spec's.
- Typed conveniences are added on top, never instead: a `Color` where a CSS
  string is accepted, enums alongside the string keywords.

### 1. `HTMLCanvasElement` (W1, W9, W10; engine + framework)

```rust
/// A typed handle to a <canvas> element (Element derefs to it by tag).
pub struct HtmlCanvasElement { /* wraps Element */ }
impl HtmlCanvasElement {
    pub fn create(document: &Document) -> Result<Self, Error>;     // document.createElement("canvas")
    pub fn from_element(element: Element) -> Option<Self>;
    pub fn width(&self) -> u32;  pub fn set_width(&self, w: u32);   // the width attribute (default 300)
    pub fn height(&self) -> u32; pub fn set_height(&self, h: u32);  // the height attribute (default 150)
    pub fn get_context_2d(&self, settings: CanvasRenderingContext2dSettings)
        -> Result<Option<CanvasRenderingContext2d>, Error>;         // getContext("2d", { alpha, willReadFrequently, colorSpace })
    pub fn get_context_webgpu(&self) -> Result<Option<GpuCanvasContext>, Error>; // getContext("webgpu")
    pub fn to_data_url(&self, mime: &str, quality: Option<f64>) -> Result<String, Error>;
    pub fn to_blob(&self, mime: &str, quality: Option<f64>) -> Result<Vec<u8>, Error>;
}
```

- **Spec semantics.** `getContext` returns `None` (HTML: `null`) when the
  canvas already has a context of another kind.
- **Sizing.** The bitmap is `width`×`height`. The element's intrinsic size
  is the bitmap size, 300×150 by default. CSS scaling follows `object-fit`
  (default `fill`) and `image-rendering`.
- **Detached and stacked canvases.** A canvas outside the document keeps a
  working context, which packages use as scratch canvases. Stacked canvases
  composite in paint order.
- **`view!`.** It accepts `<canvas width=… height=…>` already. A typed
  handle needs `node_ref` in `view!`, which sample-git also needs, and
  which decision 12 accepts as framework work:
  `<canvas node_ref=canvas_ref …>` with
  `canvas_ref: NodeRef<HtmlCanvasElement>`.

### 2. `CanvasRenderingContext2D` (W2–W8; engine, crate `div-canvas`)

The members of HTML's `CanvasRenderingContext2D` and its mixins,
implemented by porting Blink's `third_party/blink/renderer/modules/canvas/canvas2d`
onto Div's Skia with Chromium's paint flags:

```rust
#[derive(Clone)] // a handle; !Send, bound to its canvas
pub struct CanvasRenderingContext2d { /* … */ }
impl CanvasRenderingContext2d {
    pub fn canvas(&self) -> HtmlCanvasElement;
    // CanvasState
    pub fn save(&self); pub fn restore(&self); pub fn reset(&self); pub fn is_context_lost(&self) -> bool;
    // CanvasTransform
    pub fn scale(&self, x: f64, y: f64); pub fn rotate(&self, angle: f64); pub fn translate(&self, x: f64, y: f64);
    pub fn transform(&self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64);
    pub fn set_transform(&self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64);
    pub fn set_transform_matrix(&self, m: &DomMatrix2dInit); pub fn get_transform(&self) -> DomMatrix;
    pub fn reset_transform(&self);
    // CanvasCompositing
    pub fn global_alpha(&self) -> f64; pub fn set_global_alpha(&self, a: f64);
    pub fn global_composite_operation(&self) -> String;
    pub fn set_global_composite_operation(&self, op: &str);         // ignored if invalid, as specified
    // CanvasImageSmoothing
    pub fn set_image_smoothing_enabled(&self, on: bool);
    pub fn set_image_smoothing_quality(&self, q: ImageSmoothingQuality);
    // CanvasFillStrokeStyles
    pub fn set_fill_style(&self, style: impl Into<CanvasStyle>);    // &str (CSS color) | Color | CanvasGradient | CanvasPattern
    pub fn set_stroke_style(&self, style: impl Into<CanvasStyle>);
    pub fn fill_style(&self) -> CanvasStyle; pub fn stroke_style(&self) -> CanvasStyle;
    pub fn create_linear_gradient(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> CanvasGradient;
    pub fn create_radial_gradient(&self, x0: f64, y0: f64, r0: f64, x1: f64, y1: f64, r1: f64) -> Result<CanvasGradient, DomException>;
    pub fn create_conic_gradient(&self, start_angle: f64, x: f64, y: f64) -> CanvasGradient;
    pub fn create_pattern(&self, image: CanvasImageSource, repetition: &str) -> Result<Option<CanvasPattern>, DomException>;
    // CanvasShadowStyles
    pub fn set_shadow_offset_x(&self, v: f64); pub fn set_shadow_offset_y(&self, v: f64);
    pub fn set_shadow_blur(&self, v: f64); pub fn set_shadow_color(&self, css: impl Into<CanvasColor>);
    // CanvasFilters (W20, later)
    pub fn set_filter(&self, css: &str);
    // CanvasRect
    pub fn clear_rect(&self, x: f64, y: f64, w: f64, h: f64);
    pub fn fill_rect(&self, x: f64, y: f64, w: f64, h: f64);
    pub fn stroke_rect(&self, x: f64, y: f64, w: f64, h: f64);
    // CanvasDrawPath
    pub fn begin_path(&self);
    pub fn fill(&self, rule: CanvasFillRule); pub fn fill_path(&self, path: &Path2d, rule: CanvasFillRule);
    pub fn stroke(&self); pub fn stroke_path(&self, path: &Path2d);
    pub fn clip(&self, rule: CanvasFillRule); pub fn clip_path(&self, path: &Path2d, rule: CanvasFillRule);
    pub fn is_point_in_path(&self, x: f64, y: f64, rule: CanvasFillRule) -> bool;
    pub fn is_point_in_stroke(&self, x: f64, y: f64) -> bool;
    // CanvasText
    pub fn fill_text(&self, text: &str, x: f64, y: f64, max_width: Option<f64>);
    pub fn stroke_text(&self, text: &str, x: f64, y: f64, max_width: Option<f64>);
    pub fn measure_text(&self, text: &str) -> TextMetrics;          // all 12 metrics, from div-text
    // CanvasTextDrawingStyles
    pub fn set_font(&self, css_font: &str);                          // CSS font shorthand; invalid → ignored
    pub fn set_text_align(&self, v: CanvasTextAlign); pub fn set_text_baseline(&self, v: CanvasTextBaseline);
    pub fn set_direction(&self, v: CanvasDirection);
    pub fn set_letter_spacing(&self, css: &str); pub fn set_word_spacing(&self, css: &str);
    pub fn set_font_kerning(&self, v: CanvasFontKerning);
    // CanvasDrawImage (W6)
    pub fn draw_image(&self, image: CanvasImageSource, dx: f64, dy: f64);
    pub fn draw_image_scaled(&self, image: CanvasImageSource, dx: f64, dy: f64, dw: f64, dh: f64);
    pub fn draw_image_sub(&self, image: CanvasImageSource, sx: f64, sy: f64, sw: f64, sh: f64,
                          dx: f64, dy: f64, dw: f64, dh: f64);
    // CanvasImageData (W7)
    pub fn create_image_data(&self, w: u32, h: u32) -> ImageData;
    pub fn get_image_data(&self, x: i32, y: i32, w: u32, h: u32) -> Result<ImageData, DomException>;
    pub fn put_image_data(&self, data: &ImageData, dx: i32, dy: i32);
    // CanvasPathDrawingStyles
    pub fn set_line_width(&self, v: f64); pub fn set_line_cap(&self, v: CanvasLineCap);
    pub fn set_line_join(&self, v: CanvasLineJoin); pub fn set_miter_limit(&self, v: f64);
    pub fn set_line_dash(&self, segments: &[f64]); pub fn get_line_dash(&self) -> Vec<f64>;
    pub fn set_line_dash_offset(&self, v: f64);
    // CanvasPath (also on Path2d, W8)
    pub fn close_path(&self); pub fn move_to(&self, x: f64, y: f64); pub fn line_to(&self, x: f64, y: f64);
    pub fn quadratic_curve_to(&self, cpx: f64, cpy: f64, x: f64, y: f64);
    pub fn bezier_curve_to(&self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64);
    pub fn arc_to(&self, x1: f64, y1: f64, x2: f64, y2: f64, r: f64) -> Result<(), DomException>;
    pub fn rect(&self, x: f64, y: f64, w: f64, h: f64);
    pub fn round_rect(&self, x: f64, y: f64, w: f64, h: f64, radii: &[RoundRectRadius]) -> Result<(), DomException>;
    pub fn arc(&self, x: f64, y: f64, r: f64, start: f64, end: f64, ccw: bool) -> Result<(), DomException>;
    pub fn ellipse(&self, x: f64, y: f64, rx: f64, ry: f64, rotation: f64, start: f64, end: f64, ccw: bool)
        -> Result<(), DomException>;
}
pub struct Path2d { /* … */ }  // new(), from_svg_path_data(&str), add_path(&Path2d, Option<&DomMatrix2dInit>) + CanvasPath
pub enum CanvasImageSource<'a> { Image(&'a Element), Canvas(&'a HtmlCanvasElement), ImageBitmap(&'a ImageBitmap) /* W19 */ }
```

**Semantics.**

- Results and `DomException` appear exactly where WebIDL throws, for
  example a negative radius in `arc`. Everywhere else, invalid input is
  ignored, as specified.
- Drawing goes immediately to a CPU raster bitmap: Chromium's unaccelerated
  canvas, which is what pinned Chromium runs with `--disable-gpu`.
- The element shows the bitmap committed at the end of the current task or
  frame. `getImageData` sees all draws so far.

### 3. Around the canvas (framework, with engine where noted)

```rust
// W11: HTML "run the animation frame callbacks", inside Document::begin_frame,
// after pending events and before style/layout/paint. Per document, typed
// handle, every window (decision 4).
impl Document {
    pub fn request_animation_frame(&self, callback: impl FnOnce(f64) + 'static) -> AnimationFrameHandle;
    pub fn cancel_animation_frame(&self, handle: AnimationFrameHandle);
}

// W12
impl Window { pub fn device_pixel_ratio(&self) -> f64; }
pub fn match_media(query: &str) -> MediaQueryList;   // "(resolution: 2dppx)" with change events

// W13: one API with three boxes; engine computes after layout, framework
// delivers before paint (decision 4).
pub struct ResizeObserver { /* … */ }
impl ResizeObserver {
    pub fn new(callback: impl Fn(&[ResizeObserverEntry], &ResizeObserver) + 'static) -> Self;
    pub fn observe(&self, target: &Element, options: ResizeObserverOptions); // box: content-box | border-box | device-pixel-content-box
    pub fn unobserve(&self, target: &Element); pub fn disconnect(&self);
}

// W14: PointerEvent fields on Event, and native mouse input producing
// pointer* events before the compatibility mouse* events, as Chromium does.
// Typed enums and bitflags carrying web values; the mouse's pointerId is 1
// (decision 4).
impl Event {
    pub fn offset_x(&self) -> f64; pub fn offset_y(&self) -> f64;   // from the hit test's world_to_local
    pub fn pointer_id(&self) -> i32;                                 // 1 for the mouse
    pub fn buttons(&self) -> Buttons;                                // bitflags with web values: PRIMARY = 1, SECONDARY = 2, AUXILIARY = 4
    pub fn pointer_type(&self) -> Option<PointerType>;               // Mouse | Pen | Touch
    pub fn pressure(&self) -> f32; pub fn tilt_x(&self) -> i32; pub fn tilt_y(&self) -> i32;
}
// plus dispatch of "dblclick", "contextmenu", "gotpointercapture"/"lostpointercapture".

// W15
pub struct FontFace { /* new(family, source, descriptors), load() -> future, status() */ }
impl Document { pub fn fonts(&self) -> FontFaceSet; } // add(), ready() -> future, check()

// testing
impl TestApp { pub fn advance_frame(&mut self, ms: f64); } // runs rAF callbacks, then renders
```

### 4. `GPUCanvasContext` (W17, W18; framework platform + engine scene)

The WebGPU API in Rust is wgpu; it is the implementation behind Firefox's
WebGPU. Div exposes `navigator.gpu` from the wgpu instance its presenter
uses, and the canvas context in WebGPU's shape:

```rust
pub fn gpu() -> Gpu;                                          // navigator.gpu
impl Gpu {
    pub async fn request_adapter(&self, options: &wgpu::RequestAdapterOptions<'_, '_>) -> Option<wgpu::Adapter>;
    pub fn get_preferred_canvas_format(&self) -> wgpu::TextureFormat;
}
pub struct GpuCanvasContext { /* … */ }
pub struct GpuCanvasConfiguration {
    pub device: wgpu::Device,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,           // default RENDER_ATTACHMENT
    pub view_formats: Vec<wgpu::TextureFormat>,
    pub alpha_mode: GpuCanvasAlphaMode,       // Opaque | Premultiplied
}
impl GpuCanvasContext {
    pub fn canvas(&self) -> HtmlCanvasElement;
    pub fn configure(&self, configuration: GpuCanvasConfiguration) -> Result<(), DomException>;
    pub fn unconfigure(&self);
    pub fn get_current_texture(&self) -> Result<wgpu::Texture, DomException>; // the texture for this frame
}
impl wgpu::Queue /* extension trait */ {
    fn copy_external_image_to_texture(&self, source: GpuCopyExternalImageSourceInfo, dest: …, size: …); // W18
}
```

**Semantics.**

- **Frame model.** As in WebGPU, the texture from `get_current_texture` is
  presented when the current frame's tasks finish. Div composites it at the
  canvas's slot, with clip, transform and opacity.
  - `alphaMode: opaque` ignores alpha.
  - `premultiplied` composites under Div content with a hole punch.
  - Translucent canvases over Div content below them are limited until
    decision 1's layers exist (design.md risk R3).
- **Shared device** (decision 5). Adapters and devices come from Div's
  own instance, so the compositor samples canvas textures on the same
  device, with no cross-API interop. This departs from browsers' per-page
  devices; see design.md risk R2.
- **Device loss.** On loss, Div recreates its device and keeps presenting
  its own content. The app sees the loss as WebGPU reports it, and
  recreates its device and resources.
- **Software presentation** reads canvas textures back. That is slow, and
  it is reported.

### 5. Internal: canvas slots (engine)

This is not public API. It is a client of the engine's one partial-redraw
design (decision 1): a slot change is a damage region, and a GPU canvas is
a layer. Paint records a slot at each canvas's content box.
Canvas contents (a 2D bitmap or a GPU texture) are versioned outside the
document lifecycle:

- A content change requests a frame whose damage is the canvas's device
  rect, with no relayout and no re-record.
- The CPU compositor re-rasters only the tiles under that rect.
- The wgpu presenter redraws the quad.

This is what lets canvas animation run without re-rendering the window.
Probe P7 measured 25–29 ms per frame today.

## Affected tracks

| Track | Work (size) |
| --- | --- |
| Engine | W1 element sizing and bitmap display (S); W2+W3 core context and styles, Blink port (L); W4 shadows (S/M); W5 text through div-text (M); W6 `drawImage` (M); W7 image data (S); W8 `Path2D` and point-in-path (M); W10 detached and stacked canvases (S); W13 resize observations after layout (half of M); W16 fallback content (S); W20 filters, layers, reset, color (L, later); canvas slots and canvas-only frames (M/L); W17 scene geometry for GPU canvases (M). |
| Framework | W9 `toDataURL`/`toBlob` (S); W11 `requestAnimationFrame` and `advance_frame` (S/M); W12 `devicePixelRatio` and `matchMedia` (S); W13 `ResizeObserver` delivery (half of M); W14 Pointer Events, `dblclick`, `contextmenu` (M); W15 `FontFace` and `document.fonts` (S/M); W17 `navigator.gpu`, `GPUCanvasContext` and a wgpu presenter (L); W18 `copyExternalImageToTexture` (S); W19 `OffscreenCanvas` and `createImageBitmap` (M, with engine); W21 WebGL2 (XL, later); a `view!` node ref (S). |
| Components | None required. |

## Migration

Every step is additive, so `main` stays green, and each lands with its
tests. Nothing changes existing rendering: a document without a canvas
context records the same picture as before.

1. **W1, W11, W12.**
   - Tests: canvas sizing against Chromium census pages, and rAF ordering
     headless.
2. **W2+W3, in slices.** Each slice reports two numbers: WPT assertions
   passing, and exact matches against pinned Chromium.
   - Core paths, fill and stroke first (431 WPT tests).
   - Line styles and dashes next.
   - Then styles: gradients and patterns.
3. **Canvas slots.**
   - Gate: a canvas-only frame equals a full re-record, pixel for pixel.
4. **W14, W13.**
   - Tests: `offsetX` under rotate and scale, pointer capture retargeting,
     and resize observations at device scale 1 and 2.
5. **W5, W7, W4, W8, W6**, each with its WPT bucket. Package call-trace
   replays (design.md §7.2) start as soon as a package's members exist.
6. **W9, W10, W15, W16.**
7. **W17, W18.** First verify that headless Chromium 147 exposes
   SwiftShader WebGPU, the oracle for the composition comparison. Then
   build them, with the wgpu presenter behind `BackendPreference` until
   it passes the CPU presenter's pixel tests. Then the WebGPU CTS subset
   and the composition comparison (design.md §7.3).
8. **W19, W20**, then W21 if the WebGL-only paths of three.js and Babylon
   are still needed.
