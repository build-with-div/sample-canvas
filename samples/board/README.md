# board

An Excalidraw-style board on Div:

- a `<canvas>` under a toolbar of five tools (selection, rectangle,
  diamond, ellipse, line);
- a style panel (stroke and background colors, stroke width, sloppiness,
  fill style);
- a layers list;
- pointer gestures that create, select and move elements;
- shortcuts: `V R D O L` or `1`–`5` pick a tool, Escape deselects, and
  Delete or Backspace removes the selection.

Each element is drawn with the rough.js port in
[samples/roughjs](../roughjs).

```
cargo run --release -p board
```

The canvas stays blank: Div has no 2D context yet (W1, W2), so the
scene's rough.js calls go to a recorder. Everything else works.

## Platform features and gaps

| Web feature the board uses | Div today | In this sample |
| --- | --- | --- |
| `<canvas>` + `CanvasRenderingContext2D` (W1, W2) | missing | `CallRecorder` (`// div-gap: W1`, `W2`) |
| `requestAnimationFrame` (W11) | missing | redraw as soon as the scene changes (`// div-gap: W11`) |
| Pointer Events (W14) | the mouse fires `mouse*` only | `mousedown`/`mousemove`/`mouseup` (`// div-gap: W14`) |
| `getBoundingClientRect`, `clientX`/`clientY` | yes | used for scene coordinates |
| CSS border shorthands | rejected by `style!` (gap 15) | twelve longhands (`// div-gap: 15`) |
| Focus on `mousedown` | a non-focusable target leaves focus as it was (gap 16) | none; listed as a known difference |

The gap numbers refer to
[docs/framework-gaps.md](../../docs/framework-gaps.md).

## How it is checked

[tools/board-reference](../../tools/board-reference) is the same board as a
web page. It has the same DOM and inline styles and the same scene model in
JavaScript, and it draws with rough.js 4.6.6 on a real canvas.
`capture.mjs` replays [the scenarios](tests/fixtures/fidelity/board-scenarios.json)
in Chrome for Testing 147.0.7727.50. [tests/fidelity_board.rs](tests/fidelity_board.rs)
replays them on Div and compares each snapshot: element boxes and
attributes, focus, tool, selection, scene and the redraw's context calls.

Measured on 13 November 2026 (`cargo test -p board --test fidelity_board -- --nocapture`):

- 59 snapshots.
- All 2,548 boxes are bit-identical, and so are all 2,548 attribute sets.
- The tool, selection and scene match in every snapshot.
- Every redraw's context calls match, every number bit for bit.
- Focus matches in 43 of 59 snapshots. The other 16 are gap 16, listed in
  [board-div-differences.txt](tests/fixtures/fidelity/board-div-differences.txt).

Not compared: pixels (no 2D context yet) and text metrics (every box has a
fixed size).
