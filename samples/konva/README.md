# konva

A port of [Konva](https://konvajs.org) 10.9.1's scene graph to Rust:

- `Transform` (`Util.ts`): translate, scale, rotate, skew, multiply,
  invert, `point`, `decompose` and the transformed bounding rect;
- the node tree (`Node.ts`, `Container.ts`, `Stage`, `Layer`, `Group`,
  `Shape`, `Rect`, `Circle`): attributes with their defaults and
  `<attr>Change` events, `add`, `remove`, `destroy`, `moveTo` and the
  z-order methods;
- transforms: `getTransform`, `getAbsoluteTransform(top)`,
  `getAbsolutePosition(top)`, `setAbsolutePosition`, `getAbsoluteScale`,
  `getAbsoluteRotation`, including `transformsEnabled`;
- `getClientRect` for shapes and containers, with stroke, non-scaling
  stroke (`strokeScaleEnabled: false`), shadows, collapsed transforms and
  `relativeTo`;
- `isVisible`, `isListening`, `getAbsoluteOpacity`, `getDepth`,
  `getAbsoluteZIndex`;
- Konva's own events: `on` and `off` with namespaces (`click.name`), the
  protected `konva` namespace, `fire` with and without bubbling,
  `cancelBubble`, and the non-bubbling `mouseenter`-style events.

This part of Konva is its own data structure and uses no web platform
feature. Konva's events here are Konva's, not DOM events: the stage turns
DOM pointer events into them, which is not ported yet.

Not ported yet, and why:

- Drawing (`drawScene`, `sceneFunc`): it needs `<canvas>` (W1) and
  `CanvasRenderingContext2D` (W2), and shadows need W4.
- Hit detection: Konva draws every shape into a hidden canvas in a unique
  color and reads the pixel under the pointer with `getImageData` (W7).
- Dragging and the stage's DOM event handling: Pointer Events (W14).
- Caching (`cache()`), filters, `clone`, selectors (`find`), and the
  object-valued attribute forms (`position`, `scale`, `offset` as
  `{x, y}`).

See [docs/phase2-plan.md](../../docs/phase2-plan.md) for when these come.

The port keeps JavaScript's evaluation order and number semantics
(`src/js.rs`): `Math.min`/`Math.max` with NaN and -0, truthiness, and
`Math.hypot` as V8 computes it. Konva caches derived values; the port
recomputes them. The cached and uncached paths do the same arithmetic in
the same order.

## How it is checked

[tests/oracle.rs](tests/oracle.rs) runs the scripts in
[tests/fixtures/cases.json](tests/fixtures/cases.json) on the port and
compares every result and every event with what Konva 10.9.1 returned for
the same script in Node, recorded in `tests/fixtures/konva-10.9.1.json` by
[tools/konva-oracle](../../tools/konva-oracle). Each event is compared by
listener, type, `target`, `currentTarget`, and the old and new values or
the added child.

The scripts (from `make_cases.py`, seeded) cover:

- 67 standalone `Transform` sequences, including singular matrices;
- 8 random trees of layers, groups, rects and circles, up to four levels
  deep, with random transforms, skews, offsets, negative sizes, strokes,
  shadows, `visible`, `listening`, `opacity` and `transformsEnabled`.
  Every query runs on every node before and after attribute changes and
  `setAbsolutePosition`;
- client rects under zero scales and non-scaling strokes;
- z-order, `moveTo`, `remove` and `destroy` sequences;
- event dispatch: bubbling, `cancelBubble`, non-listening ancestors,
  namespaces, the `konva` namespace, and change events.

Measured on 13 November 2026 on Windows with Node 22.18.0
(`cargo test --release -p konva --test oracle -- --nocapture`):

- 5,832 results and 122 events: the same structure, strings, booleans,
  nodes and event order.
- 17,301 of 17,301 numbers are bit-identical (no number needed the
  2-ulp tolerance).
- `js::hypot` matches Node's `Math.hypot` on 4,000 of 4,000 seeded pairs,
  bit for bit.

A negative check: letting events bubble past `cancelBubble` fails the test
at the first such dispatch.

## Konva's behavior the port keeps

- A layer's width and height are its stage's; setting them does nothing.
  A layer without a stage has none (`undefined` in Konva, NaN in the port).
- `setZIndex` with an index out of range is clamped as
  `Array.prototype.splice` clamps it.
- `off()` with no argument keeps listeners in the `konva` namespace.
