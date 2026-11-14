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
| P2-4 | Port Konva 10.9.1's core: `Transform`, the node tree (`Stage`, `Layer`, `Group`, `Shape`), attributes, absolute transforms, `getClientRect`, event bubbling, and drag state | The scene graph and events, tested headless | Transforms, client rects and draw traces compared with Konva 10.9.1 in Node, driving a recording proxy context | W2, W7 (the color-key hit canvas), W4, W10 |
| P2-5 | A Konva demo shell in Div: stage container, layer panel, inspector | The shell and input | Harness comparison of the shell against an HTML equivalent | W1, W2, W14 |
| P2-6 | A three.js WebGPU example shell: toolbar, resizable panes, and a popover over a `<canvas>` viewport | The shell; the scene runs on wgpu offscreen in a test | Shell comparison; the scene waits for W17 | W17, W13 |

## Swap-ins when Div lands features

| Div lands | Sample change |
| --- | --- |
| W11 `requestAnimationFrame` (Tier 0.10) | The board's redraw moves to rAF. `frame_loop` is re-measured on it. |
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
