# Framework gaps found by sample-canvas

Each gap lists:

- the web platform feature whose absence causes it, with its ID from
  [design.md §2](design.md#2-the-platform-div-must-implement);
- the owning Div track;
- the source at Div `565db46a`, unchanged at `92ea25a0` (only `view!` children changed between them);
- the temporary scaffolding, if any, that unblocks a sample until the
  feature lands. Scaffolding is labeled `// div-gap: W…` in code and
  removed when the feature arrives; it is never the solution.

## Gaps

| # | Gap | Platform feature (ID) | Track | Source | Temporary scaffolding |
| --- | --- | --- | --- | --- | --- |
| 1 | `<canvas>` paints nothing; there is no `getContext`. | `HTMLCanvasElement` (W1), `CanvasRenderingContext2D` (W2–W8) | engine | `div-paint/src/painter.rs:6331` | None for drawing (no private context). `SurfaceBridge` in `docs/probes` is a measurement tool. `samples/roughjs` has `RoughCanvasContext` and `CallRecorder` (`// div-gap: W2`), which record rough.js's context calls and draw nothing; `samples/board` redraws its scene into a `CallRecorder`. `samples/konva` has `Context2d` and its own `CallRecorder` (`// div-gap: W2`), which record Konva's scene and hit canvas calls; Konva's isolation canvas, which needs `getTransform` (W2) and `drawImage` of a canvas (W6), is not ported (`// div-gap: W2 … W6`), nor its farbling check, which reads pixels back (`// div-gap: W7`). `samples/konva-shell` draws through that recorder, and its stroked shapes set `perfectDrawEnabled: false` so that Konva never reaches the isolation canvas, in its reference page too (`// div-gap: W2 W6`). |
| 2 | A `<canvas>` with no CSS size is 300×0 inline, and fills its container's width at height 0 as a block. Chromium gives 300×150 both ways: the element has no intrinsic size until it gets an image. | `HTMLCanvasElement` sizing (W1) | engine | `div-layout/src/intrinsic_sizing.rs:3465, 3505`; `div-dom/src/tree.rs:32-39` (`TransparentCanvas` is only set in tests) | Set the canvas's CSS width and height (P1) |
| 3 | The only way to show app pixels is the undocumented `image/x-openui-rgba8` resource. | canvas bitmap display (W1) | engine | `div-paint/src/image_resource.rs:336-372` | `SurfaceBridge` (`// div-gap: W1`) |
| 4 | Image resources are cached by the `sha256` string alone and never freed, so each pushed frame leaks. | canvas bitmap display (W1); resource lifetime for `<img>` too | engine | `image_resource.rs:14-20, 179`; `div-dom/src/tree.rs:1042-1067` | A unique key per frame; samples cap their run time (P3) |
| 5 | `set_image_resource` marks the element intrinsic-dirty, so each canvas frame relayouts. | canvas slots: a content change is not a layout change (design.md §5.1), built on the engine's partial redraw (Tier 0 item 0.7, decision 1) | engine | `div-engine/src/lib.rs:2199` | None |
| 6 | Any visual change re-records the whole document and re-rasters the whole window; the damage is always the full viewport. | damage tracking and partial raster (Tier 0 item 0.7, scheduled), then canvas slots on top of it (§5.1) | engine | `div-engine/src/lib.rs:2673-2691`; `div-compositor/src/lib.rs:333-357` | None (P7 measures it) |
| 7 | There is no `requestAnimationFrame`. | `requestAnimationFrame` (W11) | framework | `divui/src/document.rs:1001-1005` | `App::on_platform_event` + `PlatformEvent::Presented`, single-window `App` only (`// div-gap: W11`). `samples/board` redraws as soon as its scene changes, where its reference page redraws in `requestAnimationFrame` (`// div-gap: W11`). `samples/konva-shell` draws a layer at once where Konva's `batchDraw` waits for a frame (`// div-gap: W11`). `samples/three-shell` runs one `controls.update()` after each step, where an app would update the controls in its animation loop (`// div-gap: W11`). |
| 8 | There is no public `devicePixelRatio`, and no notification when it changes. | `devicePixelRatio` + `matchMedia` resolution (W12) | framework | `div-platform/src/lib.rs:345` (`Resized` only) | `PlatformEvent::Resized` via `on_platform_event` (`// div-gap: W12`). `samples/konva` gives its canvases a pixel ratio of 1, as Konva has without a window (`// div-gap: W12`). |
| 9 | There is no `ResizeObserver`. | `ResizeObserver` with `devicePixelContentBoxSize` (W13) | engine + framework | (none) | `samples/three-shell` runs its viewport's resize (camera aspect, canvas size) after mounting and after each splitter move, and acts only when the size changed, where its reference page observes the viewport (`// div-gap: W13`) |
| 10 | Events carry viewport coordinates only: no `offsetX`/`offsetY`, `buttons`, `pointerType` or `pressure`. | Pointer Events (W14) | engine + framework | `divui/src/events.rs:175-191`; `div-engine/src/lib.rs:2731-2745` | Viewport minus `getBoundingClientRect` (`bounding_rect`), which is wrong under transforms (P6; `// div-gap: W14`) |
| 11 | A real mouse produces only `mouse*` events; `pointer*` events need a nonzero pointer id. Decision 4 sets the mouse's `pointerId` to 1, as in Chromium. | Pointer Events (W14) | framework | `divui/src/document.rs:1068`; `div-platform/src/desktop.rs:973` | Listen to `mouse*` (`// div-gap: W14`); `samples/board` does, and `samples/three-shell` feeds OrbitControls `mousedown`/`mousemove`/`mouseup` with `pointerId` 1 |
| 12 | There are no `contextmenu` or `dblclick` events. | Pointer and UI events (W14) | framework | (not dispatched) | None needed so far: OrbitControls cancels the canvas's context menu, and `samples/three-shell` has none to cancel (`// div-gap: 12`) |
| 13 | Neither presenter can composite a GPU texture: GL uploads the CPU frame, and the frame has no alpha. | `GPUCanvasContext` (W17) | framework + engine | `div-platform/src/desktop.rs:251-321` | wgpu scene rendered offscreen and read back into `SurfaceBridge` (`// div-gap: W17`), for the `three-webgpu` sample only |
| 14 | There is no `navigator.gpu` equivalent: Div owns no wgpu instance or device to share. Decision 5 settles a shared device with device-loss recovery. | `navigator.gpu` (W17) | framework | (none) | The sample creates its own wgpu device (`// div-gap: W17`) |
| 15 | `style!` and `style:` reject the `border-width`, `border-style` and `border-color` shorthands, and take a border width only as a unitless integer (`1`, not `1px`). Already logged as sample-git gap 4. | CSS parsing as Chromium does it: the border shorthands and `<length>` border widths | engine (style) + framework (macros) | `div-macros` style grammar (compile error "unknown Div style property `border-width`"; "invalid literal `1px` … (expected i32)") | `samples/board` writes the twelve longhands with `1` (`// div-gap: 15`), and so does `samples/konva-shell` |
| 16 | A `mousedown` on an element that can't take focus leaves focus where it was. Chromium 147 clears it: after clicking a toolbar button and then pressing on the `<canvas>`, `document.activeElement` is `body` (measured by `samples/board`'s fidelity test, 16 of 59 snapshots, `samples/konva-shell`'s, 6 of 52, and `samples/three-shell`'s, 18 of 56). | Focus on `mousedown` (the HTML focusing steps run for the click target's focusable area, or the viewport) | framework | `divui/src/document.rs:2377-2385` (`focus_from` ignores `NotFocusable`) | None; listed in `board-div-differences.txt` and both samples' `shell-div-differences.txt` |
| 17 | There is no `background` shorthand and no `user-select` property: `StyleProperty::from_css_name` knows neither, so `style!` rejects them and an element can't be given them. Konva assigns `style.background = 'transparent'` to each canvas and `style.userSelect = 'none'` to its stage's content element. | CSS Backgrounds `background` shorthand; CSS UI `user-select` | engine (style) + framework (macros) | `div-style/src/generated_properties.rs` (`from_css_name`: no `background`, no `user-select`) | `samples/konva-shell` sets `background-color: transparent`, which computes the same as the shorthand there, and leaves `user-select` out (`// div-gap: 17`) |
| 18 | There is no `touch-action` property: `StyleProperty::from_css_name` doesn't know it. OrbitControls sets `style.touchAction = 'none'` on its element so that touch drags don't scroll. | CSS `touch-action` (with touch input, W14) | engine (style) + framework (touch input) | `div-style/src/generated_properties.rs` (`from_css_name`: no `touch-action`) | `samples/three-shell` leaves the canvas's style as it is (`// div-gap: 18`); the shell takes no touch input, so nothing compared depends on it |
| 19 | A `wheel` event has no `deltaMode`, and its `deltaX` and `deltaY` are `f32`, where Chromium's are doubles. OrbitControls scales line and page deltas by `deltaMode`. | UI Events `WheelEvent` (W14) | framework | `divui/src/events.rs:333-338` (`Event::wheel`) | `samples/three-shell` gives OrbitControls `deltaMode` 0 (`// div-gap: 19`). The scripted wheel events have whole-pixel deltas, and the camera after each matches Chromium's bit for bit, so Chromium's `deltaMode` was 0 there too; a line-mode device or a fractional delta would differ |
| 20 | Key events carry `key` only: there is no `code`, and a modifier key alone (Control, Shift) produces no `Key`. OrbitControls reads `code` for the arrow keys and watches Control on the document to tell a pinch from a ctrl-wheel. | UI Events `KeyboardEvent.code` and modifier `key` values | framework | `divui/src/keyboard.rs` (`Key` has no modifier keys); `divui/src/events.rs` (no `code`) | `samples/three-shell` maps the arrow keys' `key` to their `code`, which has the same name, and doesn't wire the Control tracking (`// div-gap: 20`) |
| 21 | Hiding the focused element (here `display: none` on the popover that holds it) leaves it focused. Chromium 147 runs the focus fixup rule: `document.activeElement` becomes `body` (measured by `samples/three-shell`'s fidelity test, 1 of 56 snapshots). | HTML focus fixup rule | engine + framework | (none: nothing unfocuses an element that loses its box) | None; listed in `samples/three-shell`'s `shell-div-differences.txt` |

## Probe results

Measured 13 November 2026 on Windows 11, with Div `565db46a` and the
release profile.

- **Machine:** 24 logical cores, 64 GB, RTX 3090, 5120-wide display at
  239 Hz, device scale 1.0 for the probe window.
- **Load:** Windows reported CPU load at 100% before and after each run,
  because other sessions were building at the same time. **The timings are
  therefore pessimistic.** They show where the cost is, not what an idle
  machine achieves.

Commands, from `docs/probes` after `. tools\win-build.ps1`:

```
cargo test --release --locked --test probes -- --nocapture --test-threads 1
E:\open-ui\out\cargo-sample-canvas\release\frame_loop.exe 3 auto
E:\open-ui\out\cargo-sample-canvas\release\frame_loop.exe 3 software
```

All 7 probe tests pass.

| Probe | Result |
| --- | --- |
| P1 | `view!` accepts `<canvas>`, and it paints nothing. **Default size:** a canvas made by `view!` measures `{x: 0, y: 14.5, w: 300, h: 0}`. One made by `Element::create` also measures 300×0. With `display: block` it measures 400×0, filling the 400 px viewport. Chromium gives 300×150 in both cases (gap 2). With an explicit height it is a normal 300×150 box. |
| P2 | An app draws with its own `skia-safe` (fill, dashed gradient stroke) and shows the result through `SurfaceBridge`. The pixels land 1:1 at the element's position, and transparent pixels show the page. |
| P3 | Re-registering under the same key returns a new `ImageResourceId(1)`, but the element still shows the old red frame: the decode cache is keyed by the string. |
| P4 | At device scale 2, a 100×50 backing store in a 50×25 CSS box maps 1:1 to device pixels. Alternating one-device-pixel columns read `[0, 255, 0, 255, …]` exactly. |
| P5 | A `z-index` popover draws over the canvas. An `overflow: hidden` scroller clips it, and `scroll_to` moves it. Ordinary replaced content composes correctly. |
| P6 | `mousedown` at viewport (110, 120) over a canvas at (100, 100) gives (10, 20) by subtracting the bounds. With `set_pointer_capture(0)`, a move and an up at (20, 20), outside the canvas, still reach it (`mousedown`, `mousemove`, `mouseup`). No `pointer*` events fire for pointer id 0. After `rotate(90deg)` the bounds become `{x: 125, y: 75, w: 50, h: 100}`, so subtracting the bounds no longer gives canvas coordinates. |
| P7 (headless) | A 640×480 canvas in a 1024×768 viewport, 60 frames, two runs. Per pushed frame: app draw 0.80 / 0.81 ms; readback, register and set 1.03 / 1.44 ms; **Div render 25.5 / 29.2 ms**. A render with no change costs 7.6 / 8.9 ms. That baseline is the whole `render_at` call with nothing changed; it is not broken down further. |
| `frame_loop` (native) | An 800×600 window with a 640×480 canvas, frames driven by `Presented`, 3 s per run. **OpenGL:** 135 frames, 44.6 fps, interval mean 22.3 ms (p50 22.2, p95 24.8); a second run gave 44.4 fps, mean 22.4 ms. **softbuffer:** 125 frames, 41.3 fps, mean 24.1 ms (p95 27.0). On a 239 Hz display both are bound by the Div frame, not vsync. The push costs 0.8 ms. |

**What the numbers say.**

- The app's own draw and the pixel handoff cost about 2 ms per frame.
- The rest of each frame is Div's full re-record and full-window raster
  (gaps 5 and 6).
- That puts a canvas animation at about 44 fps in an 800×600 window on this
  machine under load. The canvas slots in design.md §5.1 target exactly this
  cost.

### Re-run at Div `92ea25a0`

Re-run on 13 November 2026, on the same machine and with the same commands,
after the pin moved from `565db46a` to `92ea25a0`. All seven probes pass, and
P1–P6 print the same values as before.

P7 measured, per frame:

- draw: 0.47 ms;
- readback, register and set: 0.66 ms;
- Div render: 17.69 ms;
- an unchanged-document render: 6.49 ms.

These are lower than the first run's 25.5 ms render. Between the two
commits Div changed only `view!` children (`div-macros`, `divui/src/view_child.rs`),
none of the render path. So the difference is machine load, which other
sessions' builds vary, not a Div change. The numbers stay pessimistic and
uncontrolled.
