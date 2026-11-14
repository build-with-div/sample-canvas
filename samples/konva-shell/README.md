# konva-shell

A Konva demo shell on Div:

- a stage of 480×360 in its container, with two layers: a background
  (two rects) and shapes (a rounded, stroked rect, a circle, and a rotated
  group holding a dashed rect and a translucent circle);
- a toolbar: add a rect or a circle, move the selection up or down, delete
  it;
- a layers panel listing every layer, group and shape, topmost first;
  clicking a row selects the node;
- an inspector for the selected node: `x`, `y`, rotation and opacity with
  − and + buttons, fill swatches for shapes, and a visibility toggle;
- keys: arrows move the selection by one pixel, Delete or Backspace
  removes it, Escape deselects.

The scene is the Konva port in [samples/konva](../konva).
[src/stage_dom.rs](src/stage_dom.rs) ports the part of Konva that touches
the page: the stage's `konvajs-content` element, each layer's `<canvas>`
with Konva's inline styles and size, and a layer's `display` when it is
hidden.

```
cargo run --release -p konva-shell
```

The stage stays blank: Div has no 2D context yet (W1, W2), so Konva's
draws go to the port's call recorder. Everything else works.

## Platform features and gaps

| Web feature the shell uses | Div today | In this sample |
| --- | --- | --- |
| `<canvas>` + `CanvasRenderingContext2D` (W1, W2) | missing | the port's `CallRecorder` (`// div-gap: W2`) |
| `getTransform()` and `drawImage` of a canvas, for Konva's isolation canvas (W2, W6) | missing | stroked shapes set `perfectDrawEnabled: false`, in the reference page too (`// div-gap: W2 W6`) |
| `requestAnimationFrame` (W11), for `layer.batchDraw()` | missing | the layer is drawn at once (`// div-gap: W11`) |
| The `background` shorthand and `user-select` | not Div style properties (gap 17) | `background-color: transparent`; no `user-select` (`// div-gap: 17`) |
| CSS border shorthands | rejected by `style!` (gap 15) | twelve longhands (`// div-gap: 15`) |
| Focus on `mousedown` | a non-focusable target leaves focus as it was (gap 16) | none; listed as a known difference |

Not used: Konva's own pointer handling on the stage. Selecting or dragging
a shape on the canvas needs hit detection, which reads the hit canvas back
(W7), and Pointer Events (W14); that is milestone P2-4c.

The gap numbers refer to
[docs/framework-gaps.md](../../docs/framework-gaps.md).

## How it is checked

[tools/konva-reference](../../tools/konva-reference) is the same app as a
web page, running Konva 10.9.1 itself on real `<canvas>` elements. Its
DOM, inline styles and app code match the shell's step for step. A
`Proxy` around each 2D context Konva gets records every call and
assignment and forwards it to Chromium's context; Konva's reads go to the
real context. `capture.mjs` replays
[the scenarios](tests/fixtures/fidelity/shell-scenarios.json) in Chrome for
Testing 147.0.7727.50. [tests/fidelity_shell.rs](tests/fidelity_shell.rs)
replays them on Div and compares each snapshot: every element's box,
attributes and text, focus, the selection, Konva's node tree with every
attribute, and the calls each layer's scene and hit canvas received.

Both sides seed `Math.random` the same way, so Konva's hit colors match.
The page turns Konva's automatic drawing off, as the port has none, and
draws with `layer.batchDraw()`.

Measured on 14 November 2026 on Windows
(`cargo test -p konva-shell --test fidelity_shell -- --nocapture`):

- 52 snapshots, from 6 scenarios of 46 clicks and key presses.
- All 2,555 boxes are bit-identical, and so are all 2,555 attribute sets
  and texts.
- The element list, selection and node tree match in all 52 snapshots.
- All 208 canvas traces match: 4,121 context calls, and all 5,728 of
  their numbers bit-identical.
- Focus matches in 46 of 52 snapshots. The other 6 are gap 16, listed in
  [shell-div-differences.txt](tests/fixtures/fidelity/shell-div-differences.txt).

Negative checks: not hiding a layer's canvas when the layer is hidden
fails one box; drawing only the scene canvas fails 30 of the 208 traces.

Not compared: pixels (no 2D context yet) and text metrics (every box has a
fixed size).
