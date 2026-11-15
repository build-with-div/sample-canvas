# Phase 2 plan

Phase 2 ports the packages named in [design.md §8](design.md) onto Div.
Each milestone is one focused PR, merged by hand when CI is green, before
the next one starts.

## What Div has scheduled

The user approved Tier 0 of the consolidated platform plan on 13 November
2026. In that plan's numbering, the items that bear on canvas are:

- framework 0.10: `requestAnimationFrame` and frame advance (W11);
- framework 0.11: `ResizeObserver` (W13);
- framework 0.12: unified Pointer Events and test input helpers (W14);
- engine 0.7: damage tracking and partial raster, the base for canvas slots;
- components 0.13: WPT script translation, needed to port the WPT
  `html/canvas` tests.

Not scheduled:

- the canvas element (C1, i.e. W1);
- Canvas 2D (C2/C3, i.e. W2–W8);
- the WebGPU canvas (C5, i.e. W17). Its design is settled by decision 5 of the 13 November platform decisions (see design.md), but the work is not scheduled.

So no sample can draw pixels yet. Both canvas routes stay behind labeled
`// div-gap:` scaffolding, and no sample contains a 2D context or
compositor of its own (design.md §9).

## Milestones

| # | PR | Runs on current Div | Fidelity measure | Gaps it exercises |
| --- | --- | --- | --- | --- |
| P2-0 | Pin divui to Div `main` (`92ea25a0`), reseed `Cargo.lock`, re-run the probes, and add this plan | Probes P1–P7 | Probe results compared before and after the bump | none new |
| P2-1 | Make the repository root a Cargo workspace. Port rough.js 4.6.6's generator: its seeded random (an LCG over `Math.imul`), geometry, fillers and ops, plus its four dependencies (`hachure-fill`, `path-data-parser`, `points-on-curve`, `points-on-path`) | All of it: pure geometry, no platform features | `OpSet` output compared with rough.js 4.6.6's own `generator` in Node 22, on the same shapes and seeds; report the maximum coordinate difference | none |
| P2-2 | Port rough.js's canvas renderer (`RoughCanvas`) against the `CanvasRenderingContext2D` member names. Until W2 exists, its only backend is a call recorder labeled `// div-gap: W2` | The renderer logic and the call trace | The call trace compared with rough.js's `RoughCanvas` driving a recording proxy context in Node, on the same input | W1, W2 (logged, no pixels) |
| P2-3 | An Excalidraw-style board shell in Div: toolbar, style panel, element list, shortcuts, and a scene model with pointer gestures. Coordinates come from `clientX`/`clientY` and `getBoundingClientRect`, as Excalidraw computes them | The shell, layout and input, tested headless | The shell compared with a matching HTML page in pinned Chromium, replayed over the DevTools protocol (the runner sample-interactions uses) | W1, W2, W11, W14; new gaps 15 (border shorthands) and 16 (mousedown focus) |
| P2-4 | Port Konva 10.9.1's scene graph: `Transform`, the node tree (`Stage`, `Layer`, `Group`, `Shape`, `Rect`, `Circle`), attributes and change events, z-order, absolute transforms and positions, `getClientRect`, visibility, and Konva's event dispatch with bubbling | All of it: the scene graph is Konva's own data structure | Every result and event compared with Konva 10.9.1 in Node on the same scripts | none |
| P2-4b | Port Konva's drawing of `Rect` and `Circle`: `drawScene` and `drawHit` for stages, layers, groups and shapes, the layer canvases, clips, fills, strokes, dashes, opacity, shadows (with Konva's color parser), and the color-key hit canvas | The scene and hit canvas call traces | Every context call compared with Konva 10.9.1 driving a recording context in Node, on the same scripts | W2 (recorded, no pixels), W6 and W2 `getTransform` (the isolation canvas, not ported), W7 (farbling check), W12 |
| P2-4c | Konva's hit detection (`getIntersection`) and dragging | Hit detection needs pixels: it reads the hit canvas back | Waits for W2 and W7; dragging for W14 | W2, W7, W14 |
| P2-5 | A Konva demo shell in Div: stage container, layers panel, inspector and toolbar over the Konva port, with Konva's DOM (the stage's content element and layer canvases) ported onto Div | The shell, its input, and Konva's draws (recorded) | The shell compared with the same app on Konva 10.9.1 itself in pinned Chromium, on its real 2D context, replayed over the DevTools protocol | W1, W2 (recorded), W2 and W6 (`perfectDrawEnabled` off), W11; new gap 17 (`background` shorthand, `user-select`). Not W14: picking shapes on the stage needs hit detection (P2-4c) |
| P2-6a | Port three.js 0.187.0's camera math (`Vector3`, `Quaternion`, `Euler`, `Matrix4`, `Spherical`), `PerspectiveCamera` and `OrbitControls` | All of it: pure math and input handling | Every result and event compared with three.js 0.187.0 running the same scripts in pinned Chromium | none |
| P2-6b | A three.js example shell: toolbar, resizable panes, and a popover over a `<canvas>` viewport, with OrbitControls on the viewport | The shell, its layout and input | The shell compared with the same page on three.js in pinned Chromium, replayed over the DevTools protocol | W11, W13, W14, W17 (canvas sizing only), 12, 15, 16; new gaps 18 (`touch-action`), 19 (`WheelEvent`), 20 (`KeyboardEvent.code`, modifier keys) and 21 (focus fixup) |
| P2-6c | The three.js scene, drawn with wgpu | The scene renders offscreen in a test | Waits for W17 to present into the `<canvas>` | W17 |

## Swap-ins when Div lands features

| Div lands | Sample change |
| --- | --- |
| W11 `requestAnimationFrame` (Tier 0.10) | The board's redraw moves to rAF, and the Konva port gets `batchDraw`, which the Konva shell uses. `frame_loop` is re-measured on it. |
| W14 Pointer Events (Tier 0.12) | `mouse*` handlers become `pointer*` handlers using `offsetX`/`offsetY`, as the packages use them. |
| W13 `ResizeObserver` (Tier 0.11) | The canvas bitmap follows its `device-pixel-content-box` size. |
| Damage and partial raster (Tier 0.7) | Re-run probe P7. |
| W1 canvas element (C1, Tier 1.6) | Probe P1 flips to 300×150; samples size the bitmap through `width`/`height`. |
| W2 2D context (C2, Tier 2.1) | The recorder is deleted and the renderers draw for real. Pixels are compared against the original packages in pinned Chromium (`--disable-gpu`). |
| W17 WebGPU canvas (C5, Tier 3) | The three.js scene presents into its `<canvas>`. |

Each swap-in deletes its scaffolding and the matching row in
[framework-gaps.md](framework-gaps.md).

## Progress

### P2-1: the rough.js generator

`samples/roughjs` ports rough.js 4.6.6's generator and its four
dependencies. It is compared with rough.js 4.6.6 itself: 92 calls cover
every shape, every fill style and the main options.

Measured on 13 November 2026, on Windows with Node 22.18.0 (V8 12.4), using
`cargo test -p roughjs --test oracle -- --nocapture`:

- All 92 calls have identical structure: sets, op sequence and data
  lengths.
- 42,876 of 42,903 numbers are bit-identical; the other 27 are within
  2 ulps.
- 525 of 532 strings are identical.

**Finding for every port of JavaScript.** V8's `Math.sin` and `Math.cos`
are not fdlibm. On 200,000 inputs, the `libm` crate (fdlibm) matches Node 22
bit for bit for `tan`, `asin` and `atan`. For `sin` and `cos`, about 0.9% of
results differ by one bit. Rust's platform functions on Windows differ more:
2.3–4.0% for `sin`, `cos`, `tan` and `asin`, and 0.02% for `atan`.

This is consistent with V8's build option `v8_use_libm_trig_functions`,
which uses glibc-derived `sin` and `cos`. That code is LGPL, so the port
does not copy it.

This is not a Div gap: Div runs no JavaScript. But every sample that ports
JavaScript math and compares bit for bit will meet it.

**Deviations from rough.js:**

- **Path data with a number after `Z`.** rough.js's path parser loops
  forever on this input; the port returns an error.
- **An empty polygon with a fill.** rough.js throws a `TypeError` reading
  its first point; the port skips the polygon.
- **No shared input points.** The port does not alias the caller's point
  arrays. rough.js's hachure rotation can leave rounding residue in an
  array the caller passed in, but it does not change rough.js's output for
  that call.

### P2-2: the rough.js canvas renderer

`samples/roughjs/src/canvas.rs` ports rough.js's `RoughCanvas`: `draw`,
the per-shape methods, `fillSketch` and `_drawToContext`, including the
`fixedDecimalPlaceDigits` rounding and the `evenodd` rule for curves,
polygons and paths.

The original calls `canvas.getContext('2d')`. Div has neither `<canvas>`
(W1) nor `CanvasRenderingContext2D` (W2), so the port takes a context
instead. Its scaffolding, both labeled `// div-gap: W2
CanvasRenderingContext2D`:

- `RoughCanvasContext`, a trait naming the 13 members rough.js uses
  (`save`, `restore`, `strokeStyle`, `fillStyle`, `lineWidth`,
  `setLineDash`, `lineDashOffset`, `beginPath`, `moveTo`, `lineTo`,
  `bezierCurveTo`, `fill`, `stroke`) with their web arguments. It is an
  interface, not a context.
- `CallRecorder`, its only implementation, which records the calls and
  draws nothing.

When Div's `CanvasRenderingContext2d` lands, `RoughCanvas` takes it and
both are deleted.

The oracle drives rough.js 4.6.6's own `RoughCanvas` in Node with a
`Proxy` as its context, which records every member access, including any
the port does not model. Measured on 13 November 2026, on Windows with
Node 22.18.0, using `cargo test -p roughjs --test canvas_oracle -- --nocapture`:

- 19 calls, 811 context accesses: identical members, order and strings.
  rough.js asks for one `'2d'` context and touches no member beyond the
  13.
- 2,765 of 2,769 numbers are bit-identical; the other 4 are within 2 ulps
  (max 3.6e-15), in one ellipse, from the `sin`/`cos` difference above.
- A path parse error: rough.js throws before touching the context; the
  port returns the error and records no calls.

This compares calls, not pixels. Pixels need W2; they are compared with
rough.js in pinned Chromium when it lands (see the swap-ins above).

No new platform gap: the renderer needs W1 and W2, already logged as gap 1.

### P2-3: an Excalidraw-style board shell

`samples/board` is a board on Div: a `<canvas>` under a toolbar of five
tools, a style panel (stroke and background colors, stroke width,
sloppiness, fill style) and a layers list. It has pointer gestures to
create, select and move elements and keyboard shortcuts (tool keys,
Escape, Delete). Scene coordinates are `clientX`/`clientY` minus the
canvas's `getBoundingClientRect()`, as Excalidraw computes them. Each
element is drawn with the rough.js port.

The scene model is the board's own, in the style of Excalidraw's elements.
It is not a port of Excalidraw.

**The reference.** `tools/board-reference/board.html` is the same board as
a web page. It has the same DOM and inline styles, implements the same
model in JavaScript, and draws with rough.js 4.6.6 on a real `<canvas>`.
Like Excalidraw, it uses pointer events and redraws in
`requestAnimationFrame`. A `Proxy` around its 2D context records every
call rough.js makes.

`tools/board-reference/capture.mjs` replays 5 scripted scenarios (54
mouse and key steps) in Chrome for Testing 147.0.7727.50 over the DevTools
protocol. Input is trusted browser input, and the page settles after each
step. The script writes a snapshot after load and after every step.
`samples/board/tests/fidelity_board.rs` replays the same input on Div and
compares the same snapshot.

Measured on 13 November 2026 on Windows, using
`cargo test -p board --test fidelity_board -- --nocapture`. Re-running the
capture gives a byte-identical fixture.

| Compared | Matches |
| --- | --- |
| `getBoundingClientRect` of every element with an id, bit for bit | 2,548 / 2,548 |
| Their attributes (`role`, `aria-*`, `data-*`, `title`, `type`) | 2,548 / 2,548 |
| Element list, tool, selection, scene elements | 59 / 59 each |
| 2D context calls of the last redraw | 59 / 59, every number bit-identical |
| Focused element | 43 / 59 |

The 16 focus differences are one behavior, logged as new gap 16 in
framework-gaps.md:

- In Chromium, pressing the mouse on the canvas, which can't take focus,
  clears focus.
- In Div, the last clicked button keeps it (`divui/src/document.rs:2377-2385`).

The test fails on any difference not listed in
`board-div-differences.txt`, and on a listed one that no longer occurs.

A negative check: changing the board's hachure gap from 4 to 4.5 times the
stroke width fails 3 trace snapshots.

**Scaffolding**, each piece labeled in `samples/board/src/shell.rs`:

- `// div-gap: W1`: the `<canvas>` has no bitmap and is sized by CSS only.
- `// div-gap: W2`: the redraw goes to a `CallRecorder`.
- `// div-gap: W11`: the scene redraws as soon as it changes, not in a
  frame callback.
- `// div-gap: W14`: the board listens for `mouse*` where the page listens
  for `pointer*`.
- `// div-gap: 15`: border longhands with unitless widths, because Div
  rejects the border shorthands (gap 15, sample-git gap 4).

What this does not show:

- Pixels. The canvas is blank on Div until W2, and the shell's own
  rendering is not compared.
- Text metrics. Every box in the board has a fixed size.

### P2-4: the Konva scene graph

`samples/konva` ports Konva 10.9.1's scene graph: `Transform`, the node
tree with attributes and `<attr>Change` events, z-order, absolute
transforms and positions, `getClientRect` (strokes, non-scaling strokes,
shadows, collapsed transforms, `relativeTo`), visibility, listening,
opacity, and Konva's own event dispatch. The milestone is split: drawing
and the hit canvas are P2-4b, and hit detection and dragging, which need
W7 and W14, are P2-4c.

The oracle (`tools/konva-oracle/oracle.mjs`) runs Konva 10.9.1 in Node
without a DOM: automatic drawing is off, and the canvases Konva creates
for its layers get a context that ignores every call. It runs scripts from
`make_cases.py` and records every result and every event.
`samples/konva/tests/oracle.rs` runs the same scripts on the port.

Measured on 13 November 2026, on Windows with Node 22.18.0, using
`cargo test --release -p konva --test oracle -- --nocapture`:

- 5,832 results and 122 events: identical structure, strings, booleans,
  nodes and event order.
- 17,301 of 17,301 numbers are bit-identical.
- The port's `Math.hypot`, written after V8's algorithm, matches Node on
  4,000 of 4,000 seeded pairs bit for bit. Konva uses it to size
  non-scaling strokes.

The first run differed in one behavior: a layer's width and height are its
stage's. The port now does the same.

A negative check: letting events bubble past `cancelBubble` fails the
test.

No platform gap: the scene graph uses no web platform feature. Konva's
events here are Konva's own; turning DOM pointer events into them needs
W14 and comes with dragging in P2-4c.

### P2-4b: Konva's drawing of rects and circles

`samples/konva/src/draw.rs` ports Konva's `draw`, `drawScene` and
`drawHit` for stages, layers, groups and shapes, with the draws Konva makes
when a layer is added to the stage or the stage is resized. It covers the
layer canvases (`clearBeforeDraw`, `imageSmoothingEnabled`, a hit canvas
released while the layer isn't listening), group clips and composite
operations, and the `Rect` and `Circle` scene functions. Those include
corner radii, fills and fill rules, strokes and non-scaling strokes,
dashes, line styles, opacity, shadows, and the hit canvas, where each
shape is filled and stroked in its color key. Shadows use Konva's own CSS
color parser (`src/color.rs`); color keys come from a seedable
`Math.random`.

Div has no 2D context, so each canvas draws through `Context2d`, a trait
naming the members Konva uses, into a `CallRecorder` that draws nothing
(`// div-gap: W2`). Not ported, each labeled:

- Konva's isolation canvas, which a shape with a fill and a stroke uses
  when it is translucent or casts a shadow. It is sized from the context's
  `getTransform()` (W2) and drawn back with `drawImage` of a canvas (W6).
  The port says when Konva would use it (`use_buffer_canvas`, compared
  with Konva) and panics if asked to draw such a shape.
- The canvas-farbling check, which reads pixels back (W7).
- The device pixel ratio (W12): canvases use 1, as Konva does without a
  window.

**The oracle.** `tools/konva-oracle/oracle.mjs` now gives every canvas
Konva creates a context that records each call and assignment, by canvas
(`l1.scene`, `l1.hit`, `s.bufferHit`). Reading a property gives back the
value last assigned, as the port's recorder does, so the traces compare
Konva's logic and not a 2D context. Each script seeds `Math.random`, so
both pick the same color keys. The context has no `getTransform`: a case
that reaches the isolation canvas stops the oracle, and the case
generator turns `perfectDrawEnabled` off for those shapes in drawn scenes.

Measured on 13 November 2026, on Windows with Node 22.18.0, using
`cargo test --release -p konva --test oracle -- --nocapture`:

- 4,883 results, 174 events and 8,677 context calls: identical
  structure, strings, booleans, nodes, event order and call order.
- 23,019 of 23,032 numbers are bit-identical. The other 13 come from one
  shape rotated by -111.1°: V8's `Math.sin` and fdlibm differ by one ulp
  there (the P2-1 finding). Twelve stay within 1 ulp; `decompose()`'s skew
  cancels and turns the last into 223 ulps. That one is listed, with its
  cause, in `samples/konva/tests/fixtures/konva-differences.txt`; the test
  fails on any unlisted difference and on a listed one that no longer
  occurs.
- `js::number_to_string` matches `String(x)` in Node on 2,000 of 2,000
  seeded numbers. The first run differed on 5: when two shortest digit
  strings are equally near, ECMAScript takes the even one and Rust's
  shortest formatting did not.
- `js::hypot` still matches `Math.hypot` on 4,000 of 4,000 pairs.

A negative check: scaling shadow blur by the larger of the two scales
instead of the smaller changes 59 numbers and fails the test.

No new platform gap. The drawing needs W2 (logged as gap 1), the
isolation canvas W6 and W2's `getTransform`, the farbling check W7, and
the pixel ratio W12 (gap 8); framework-gaps.md lists the new scaffolding
under gaps 1 and 8.

### P2-5: a Konva demo shell

`samples/konva-shell` is a Konva app on Div. It has a 480×360 stage in
its container with two layers, a toolbar (add a rect or a circle, move the
selection up or down, delete it), a layers panel listing every node
topmost first, and an inspector. The inspector edits `x`, `y`, rotation,
opacity, fill and visibility. Arrow keys move the selection, Delete
removes it and Escape deselects. The scene is the Konva port.
`src/stage_dom.rs` ports the part of Konva that touches the page:
`Stage._buildDOM` and `_resizeDOM`, the canvas element `Stage.add` appends
with the inline styles and size from `Canvas.ts`, and
`Layer._checkVisibility`.

**The reference.** `tools/konva-reference/shell.html` is the same app as a
web page, on Konva 10.9.1 itself. It has the same DOM and inline styles and
the same app code, step for step. Unlike P2-4b's oracle, Konva draws on
Chromium's real 2D contexts: a `Proxy` around each context records every
call and assignment and forwards it, and Konva's reads (such as
`imageSmoothingEnabled` around a resize) go to the real context. Both
sides seed `Math.random` alike, so the hit colors match. The page turns
`Konva.autoDrawEnabled` off, as the port has no automatic drawing, and
draws with `layer.batchDraw()`.

`tools/konva-reference/capture.mjs` replays 6 scenarios (46 clicks and key
presses) in Chrome for Testing 147.0.7727.50, with the runner P2-3 uses.
`samples/konva-shell/tests/fidelity_shell.rs` replays them on Div and
compares a snapshot after load and after every step.

Measured on 14 November 2026 on Windows, using
`cargo test -p konva-shell --test fidelity_shell -- --nocapture` (and in
the full debug run). Re-running the capture gives a byte-identical
fixture.

| Compared | Matches |
| --- | --- |
| `getBoundingClientRect` of every element, including Konva's content `<div>` and layer `<canvas>` elements, bit for bit | 2,555 / 2,555 |
| Their attributes (`role`, `aria-*`, `title`, `type`, `class`, `width`, `height`) | 2,555 / 2,555 |
| Text of every element without child elements | 2,555 / 2,555 |
| Element list, selection, Konva node tree with every attribute | 52 / 52 each |
| Calls on each layer's scene and hit canvas since the last snapshot | 208 / 208: 4,121 calls, all 5,728 numbers bit-identical |
| Focused element | 46 / 52 |

The 6 focus differences are gap 16, as in the board. They are listed in
`shell-div-differences.txt`; the test fails on any unlisted difference
and on a listed one that no longer occurs.

Negative checks: not hiding a hidden layer's canvas fails one box, and
drawing only the scene canvas fails 30 of the 208 traces.

**Scaffolding**, each piece labeled:

- `// div-gap: W2` (in `samples/konva`): Konva draws into the port's
  `CallRecorder`.
- `// div-gap: W2 W6`: stroked shapes set `perfectDrawEnabled: false`, in
  the shell and in the reference page alike, because a translucent shape
  with a fill and a stroke is drawn through Konva's isolation canvas,
  which the port can't draw.
- `// div-gap: W11`: `batchDraw` waits for an animation frame in
  Chromium; the shell draws the layer at once.
- `// div-gap: 15`: border longhands with unitless widths.
- `// div-gap: 17`: Konva assigns `style.background = 'transparent'` and
  `style.userSelect = 'none'`. Div has neither property; the canvas gets
  `background-color: transparent`, which computes the same, and the
  content gets no `user-select`.

New platform gap 17: Div has no `background` shorthand and no
`user-select` (neither is a `StyleProperty`).

What this does not show:

- Pixels. The stage is blank on Div until W2.
- Konva's pointer handling on the stage. Picking a shape on the canvas
  reads the hit canvas back (W7), and dragging needs Pointer Events (W14);
  both are P2-4c. The scenarios never press on the stage.
- Text metrics. Every box has a fixed size.

### P2-6a: three.js math, camera and OrbitControls

`samples/three` ports three.js 0.187.0's `Vector3`, `Quaternion`, `Euler`,
`Matrix4` and `Spherical` (the operations a camera and its controls use),
`PerspectiveCamera`, and `OrbitControls` with mouse, wheel and keyboard
input, damping, auto-rotation and every limit. P2-6 was split: P2-6b is the
shell around a viewport, and P2-6c is the scene, which waits for W17.

**The oracle runs in Chromium, not Node.** `tools/three-oracle/make_cases.py`
writes 42 seeded scripts (2,420 operations): 725 math operations, 160
camera operations, and 40 OrbitControls sessions (1,535 operations) of
drags, wheels, keys, direct calls, option changes and updates. `tools/three-oracle/oracle.mjs`
serves `run.mjs` to a page in Chrome for Testing 147.0.7727.50, which runs
the scripts on three.js 0.187.0 and returns every result. OrbitControls'
element, document and window are stubs there, so the only part of
Chromium the oracle uses is V8 and its `Math`. Real input is P2-6b's.

The first oracle ran in Node 22 and showed that Node is not a Chromium
oracle for this sample:

- Node's V8 computes `Math.sin` and `Math.cos` with fdlibm. On 20,003
  arguments, 1,389 of Node 22.18.0's 40,006 sines and cosines differed
  from the correctly rounded values, and 377 arguments had a sine or
  cosine that differed from `libm`'s. Chromium 147's are correctly rounded
  except for some near-ties (the sample-charts session measured 33 of
  26,018). This agrees with P2-1's finding.
- On the 34 `Math.pow` calls of six OrbitControls sessions, Chromium 147
  matched the correctly rounded value every time. `libm::pow` was 1 ulp
  off on 3 of them.

So `samples/three/src/jsmath.rs` computes `sin`, `cos` and `pow` in
double-double arithmetic and rounds once. It uses Cody-Waite reduction
and Taylor series, and a Newton step for the logarithm. The code is our
own; no glibc (LGPL) code was read. Replaying the 2,867 `Math` calls
Chromium made in six diverging sessions through the port's functions
gave:

- `tan`, `asin`, `acos`, `atan2` (from `libm`) and `sqrt`: all
  bit-identical;
- `pow`: all bit-identical;
- `sin` and `cos`: all bit-identical except for four arguments, which
  Chromium rounds away from the correct result. The exact values lie
  0.4918–0.4960 ulp from the port's results, which are correctly rounded.

Measured on 14 November 2026 on Windows, using
`cargo test -p three --test oracle -- --nocapture` (and in the full debug
run). Re-recording the cases and the oracle gives byte-identical fixtures.

| Compared | Matches |
| --- | --- |
| Numbers (matrices, quaternions, Euler angles, positions, targets, polar and azimuth angles, distances, zoom) | 71,208 / 71,415 bit-identical; 125 within 2 ulps (largest difference 8.9e-16); 82 listed |
| Other values (OrbitControls state, `update()` return values, the 1,158 `change`, `start` and `end` events in order) | 1,375 / 1,375 |

The 82 listed differences come from two of Chromium's near-tie roundings:
`cos(-1.4277848129543687)` in one session and `sin(-0.34670702622915633)`
in another. In each, the camera carries a 1-ulp difference into every
later operation. The translation of `matrixWorldInverse`, a difference of
nearly equal terms, shows it most. They are listed with their cause in
`three-differences.txt`. The test fails on any unlisted difference and on
a listed one that no longer occurs. The other two near-ties stay within
2 ulps.

Negative checks: scaling `deltaMode` 1 wheel deltas by 15 instead of 16
fails the test, as does going back to `libm`'s `sin` and `cos` in
`math.rs`.

No platform gap: this milestone uses none. Matching Chromium's `Math`
is not Div's work (Div runs no JavaScript), but every port compared with
Chromium bit for bit needs it. Chromium's near-tie roundings stay a
recorded fidelity difference. The library that produces them is LGPL,
so the port doesn't copy it.

What this does not show:

- Touch input, `zoomToCursor`, and orthographic cameras: not ported.
- Real input events and the canvas element: P2-6b.
- Pixels: P2-6c, waiting for W17.

### P2-6b: a three.js example shell

`samples/three-shell` is a three.js viewer shell on Div:

- a toolbar: Reset, Auto-rotate, Damping, Settings, and a status line;
- an objects pane whose rows move the controls' target to an object;
- a splitter that resizes the pane and the viewport, clamped to
  140–360 px;
- a viewport holding a `<canvas>`, which the P2-6a port of OrbitControls
  drives with the mouse, the wheel and the arrow keys;
- a settings popover over the canvas: rotate speed, screen-space panning,
  Close, and Escape.

As in a three.js example, the viewport's size sets the camera's aspect and
the canvas's size, as `renderer.setSize` writes it.

**The reference.** `tools/three-reference/shell.html` is the same app as a
web page, on three.js 0.187.0 itself. It has the same DOM and inline styles
and the same app code, step for step. OrbitControls listens on the real
`<canvas>`, so Chromium's own pointer, wheel and key events drive it. A
`ResizeObserver` on the viewport resizes the camera and canvas.

`tools/three-reference/capture.mjs` serves the page from `tools/` (its
modules can't load from `file:`). It replays 4 scenarios (52 steps) in
Chrome for Testing 147.0.7727.50, with the runner P2-3 uses. The steps are
left, right, middle and shift drags, wheels, arrow keys with and without
Shift, clicks, splitter drags and idle frames. After each step the page
settles, then runs one animation frame, `controls.update()`.
`samples/three-shell/tests/fidelity_shell.rs` replays the same steps on
Div and compares a snapshot after load and after every step.

Measured on 14 November 2026 on Windows, using
`cargo test -p three-shell --test fidelity_shell -- --nocapture` (and in
the full debug run). Re-running the capture gives a byte-identical fixture.

| Compared | Matches |
| --- | --- |
| `getBoundingClientRect` of every element, bit for bit | 1,288 / 1,288 |
| Their attributes (`role`, `aria-*`, `type`, the canvas's `width` and `height`) | 1,288 / 1,288 |
| Text of every element without child elements | 1,288 / 1,288 |
| Element list, selected object, the controls' events in order (48 events) | 56 / 56 each |
| Camera (position, quaternion, aspect, zoom, projection matrix) and controls (target, state, rotate speed, flags) | 1,848 / 1,848 values; all 1,680 numbers bit-identical |
| Focused element | 37 / 56 |

The 19 focus differences are listed in `shell-div-differences.txt`. The
test fails on any unlisted difference and on a listed one that no longer
occurs.

- 18 are gap 16: a press on the canvas or an object row clears focus in
  Chromium and leaves it in Div.
- 1 is new gap 21: clicking Close focuses it and hides the popover that
  holds it. Chromium's focus fixup moves focus to the body; Div keeps it
  on the hidden button.

Negative checks: a 150 px minimum pane width fails 28 boxes, 8 attributes
and 8 camera numbers. A stale canvas `clientHeight` for the controls fails 236 camera
and 60 controls values, because panning reads it.

**Scaffolding**, each piece labeled:

- `// div-gap: W14`: the controls get `mousedown`/`mousemove`/`mouseup`
  (with `pointerId` 1), where the page's OrbitControls listens for
  `pointer*`. The splitter does the same.
- `// div-gap: W13`: the shell resizes the camera and canvas after mounting
  and after each splitter move, when the size changed. The page uses a
  `ResizeObserver`.
- `// div-gap: W11`: one `controls.update()` per step, on both sides, where
  an app would update in its animation loop. It keeps damping and
  auto-rotation deterministic.
- `// div-gap: W17`: no renderer draws, on either side. Both size the
  canvas as `renderer.setSize` would.
- `// div-gap: 12`: OrbitControls cancels the canvas's `contextmenu`; Div
  has none.
- `// div-gap: 15`: border longhands with unitless widths.
- `// div-gap: 18`: OrbitControls sets `touch-action: none`; Div has no
  such property.
- `// div-gap: 19`: Div's wheel events have no `deltaMode`; the controls
  get 0.
- `// div-gap: 20`: Div's key events have no `code` (the arrows' `key` is
  used) and no key for Control alone.

New platform gaps:

- 18: no `touch-action`.
- 19: `WheelEvent` has no `deltaMode`, and its deltas are `f32`.
- 20: `KeyboardEvent` has no `code`, and a modifier key alone has no
  `Key`.
- 21: no focus fixup when the focused element is hidden.

What this does not show:

- Pixels. The viewport is blank on Div until W17 (P2-6c).
- Touch input and pinch zoom: OrbitControls' touch paths are not ported.
- Device pixel ratios other than 1 (W12).
