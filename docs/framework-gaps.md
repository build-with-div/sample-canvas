# Framework gaps found by sample-canvas

Each gap lists the owning Div track, the source at Div `565db46a`, how to
reproduce it, and the workaround used here (marked `// div-gap:` in code).
Piece IDs (E1, F2, …) refer to [design.md §5](design.md#5-who-owns-what).

## Gaps

| # | Gap | Track | Piece | Source | Workaround |
| --- | --- | --- | --- | --- | --- |
| 1 | `<canvas>` paints nothing; there is no 2D context or surface API. | engine + framework | E3, F2 | `div-paint/src/painter.rs:6331` | App-side Skia plus `SurfaceBridge` (P2) |
| 2 | The only way to show app pixels is the undocumented `image/x-openui-rgba8` resource. | engine | E1 | `div-paint/src/image_resource.rs:336-372` | `SurfaceBridge` |
| 3 | Image resources are cached by the `sha256` string alone and never freed. Each pushed frame needs a fresh key, and every frame leaks. | engine | E7, E1 | `image_resource.rs:14-20, 179`; `div-dom/src/tree.rs:1042-1067` | A unique key per frame; samples cap their run time (P3) |
| 4 | `set_image_resource` marks the element intrinsic-dirty, so each canvas frame relayouts. | engine | E1 | `div-engine/src/lib.rs:2199` | None |
| 5 | Any visual change re-records the whole document and re-rasters the whole window. The damage is always the full viewport. | engine | E2 | `div-engine/src/lib.rs:2673-2691`; `div-compositor/src/lib.rs:333-357` | None (P7 measures the cost) |
| 6 | There is no `request_animation_frame` or frame callback. | framework | F1 | `divui/src/document.rs:1001-1005` | `App::on_platform_event` + `PlatformEvent::Presented`, single-window `App` only (`frame_loop`) |
| 7 | There is no public window handle, scale factor or window size, and no resize callback. | framework | F5 | `div-platform/src/lib.rs:169-210`; `divui/src/dialog.rs:140-155` | `PlatformEvent::Resized` via `on_platform_event` |
| 8 | Events carry viewport coordinates only. There are no offset coordinates, `buttons`, pointer type or pressure. | engine + framework | E4, F3 | `divui/src/events.rs:175-191`; `div-engine/src/lib.rs:2731-2745` | Viewport minus `bounding_rect`, which is wrong under transforms (P6) |
| 9 | A real mouse produces only `mouse*` events; `pointer*` events need a nonzero pointer id. | framework | F3 | `divui/src/document.rs:1068`; `div-platform/src/desktop.rs:973` | Listen to `mouse*` |
| 10 | There are no `contextmenu`, `dblclick` or `resize` events. | framework | F3 | (not dispatched) | None |
| 11 | The `x-openui-rgba8` transport is unpremultiplied. A translucent canvas loses precision: Skia premultiplies, the app unpremultiplies, Div premultiplies again. | engine | E1 | `image_resource.rs:368-372` | None |
| 12 | Neither presenter can composite a GPU texture: GL uploads the CPU frame, and the frame has no alpha. | framework | F4 | `div-platform/src/desktop.rs:251-321` | CPU readback (A-CPU) |
| 13 | There is no public Skia. An app links `skia-safe` 0.94 itself under Div's patch table. | framework | (RFC §4) | `divui/src/lib.rs` | Direct `skia-safe` dependency (P2) |
| 14 | A `<canvas>` with no CSS size is 300×0 inline, and fills its container's width at height 0 as a block. Chromium gives 300×150 both ways: the element has no intrinsic size until it gets an image. | engine | E8 | `div-layout/src/intrinsic_sizing.rs:3465, 3505`; `div-dom/src/tree.rs:32-39` (`TransparentCanvas` is only set in tests) | Always set the canvas's CSS width and height (P1) |

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
| P1 | `view!` accepts `<canvas>`, and it paints nothing. **Default size:** a canvas made by `view!` measures `{x: 0, y: 14.5, w: 300, h: 0}`. One made by `Element::create` also measures 300×0. With `display: block` it measures 400×0, filling the 400 px viewport. Chromium gives 300×150 in both cases (gap 14). With an explicit height it is a normal 300×150 box. |
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
  (gaps 4 and 5).
- That puts a canvas animation at about 44 fps in an 800×600 window on this
  machine under load. Pieces E1 and E2 in design.md target exactly this
  cost.

