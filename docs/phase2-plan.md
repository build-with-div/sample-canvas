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
| P2-3 | An Excalidraw-style board shell in Div: toolbar, style panel, element list, shortcuts, and a scene model with pointer gestures. Coordinates come from `clientX`/`clientY` and `getBoundingClientRect`, as Excalidraw computes them | The shell, layout and input, tested headless | The shell compared with a matching HTML page in pinned Chromium, through the harness, where Div supports the CSS | W1, W2, W11, W14 (gaps logged) |
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
