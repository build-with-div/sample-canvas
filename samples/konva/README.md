# konva

A port of [Konva](https://konvajs.org) 10.9.1 to Rust, in two parts.

**The scene graph:**

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

**Drawing rects and circles** (`src/draw.rs`):

- `draw`, `drawScene` and `drawHit` for stages, layers, groups and shapes,
  including the draws Konva makes when a layer is added to the stage and
  when the stage is resized;
- each layer's scene canvas and hit canvas, sized from the stage, with
  `clearBeforeDraw`, `imageSmoothingEnabled` and the hit canvas released
  while the layer isn't listening;
- group clips (`clipX`, `clipY`, `clipWidth`, `clipHeight`) and composite
  operations;
- `Rect` (with `cornerRadius`, one or four) and `Circle` scene functions;
  fill, `fillRule`, `fillEnabled`, `fillAfterStrokeEnabled`; stroke,
  `strokeWidth`, non-scaling strokes, dashes, line cap, join and miter
  limit; opacity; shadows, with Konva's own color parser
  (`Util.colorToRGBA`: names, hex, `rgb()`, `hsl()`);
- the hit canvas: each shape filled and stroked in its color key, with
  `hitStrokeWidth`. Color keys come from `Math.random`
  (`set_math_random` seeds it) and are unique among live shapes.

Each canvas's context is a `CallRecorder` (`src/context.rs`) behind
`Context2d`, a trait naming the `CanvasRenderingContext2D` members Konva
uses. Div has no 2D context yet, so nothing is drawn: the recorder keeps
the calls (`// div-gap: W2`). When Div's context lands it takes the
recorder's place.

Not ported yet, and why:

- Konva's isolation canvas: a shape with both a fill and a stroke, drawn
  translucent or with a shadow, is drawn into a second canvas first and
  copied back with `drawImage` of a canvas (W6), at a size taken from the
  context's `getTransform()` (W2). `use_buffer_canvas()` says when Konva
  would do this; the port panics if asked to draw such a shape. Konva's
  `perfectDrawEnabled: false` turns it off. Isolated groups are the same.
- Hit detection: Konva reads the hit canvas's pixel under the pointer with
  `getImageData` (W7). It also reads pixels back once to detect canvas
  farbling; the port takes the unfarbled color grid (`// div-gap: W7`).
- Dragging and the stage's DOM event handling: Pointer Events (W14).
- The device pixel ratio: Div exposes none (W12), so canvases use 1, as
  Konva does without a window (`// div-gap: W12`).
- Konva's DOM: the stage's content element and each layer's `<canvas>`.
  This crate doesn't depend on Div; that part is ported in
  [samples/konva-shell](../konva-shell) (`src/stage_dom.rs`).
- Caching (`cache()`), filters, gradients, patterns, custom `sceneFunc`,
  `hitFunc` and `clipFunc`, `clone`, selectors (`find`), and the
  object-valued attribute forms (`position`, `scale`, `offset` as
  `{x, y}`).

See [docs/phase2-plan.md](../../docs/phase2-plan.md) for when these come.

The port keeps JavaScript's evaluation order and number semantics
(`src/js.rs`): `Math.min`/`Math.max` with NaN and -0, `Math.round`,
truthiness, `Math.hypot` as V8 computes it, and `Number#toString`, which
Konva's shadow colors are written with. Konva caches derived values; the
port recomputes them. The cached and uncached paths do the same
arithmetic in the same order.

## How it is checked

[tests/oracle.rs](tests/oracle.rs) runs the scripts in
[tests/fixtures/cases.json](tests/fixtures/cases.json) on the port and
compares them with what Konva 10.9.1 did for the same scripts in Node,
recorded in `tests/fixtures/konva-10.9.1.json` by
[tools/konva-oracle](../../tools/konva-oracle). It compares every result;
every event, by listener, type, `target`, `currentTarget`, and the old and
new values or the added child; and every call and property assignment
Konva makes on each canvas's 2D context, in order.

In the oracle, each canvas Konva creates gets a context that records
calls the same way the port's `CallRecorder` does: reading a property
gives back the value last assigned. So the traces compare Konva's logic,
not a 2D context; a real context also normalizes colors and resets on
resize. Each script seeds `Math.random`, so the color keys are the same.

The scripts (from `make_cases.py`, seeded) cover:

- 67 standalone `Transform` sequences, including singular matrices;
- 8 random trees of layers, groups, rects and circles, up to four levels
  deep, with random transforms, skews, offsets, negative sizes, fills,
  strokes, dashes, line styles, corner radii, shadows in 19 color forms,
  clips, composite operations, `visible`, `listening`, `opacity` and
  `transformsEnabled`. Every query runs on every node, before and after
  attribute changes, `setAbsolutePosition` and drawing; then the stage is
  resized and a layer redrawn;
- every fill, stroke, hit and shadow style above on rects and circles in
  a clipped, rotated group and a scaled one, redrawn after changes,
  `listening` and `clearBeforeDraw` toggles, a destroyed shape and a
  stage resize;
- when Konva would use its isolation canvas, on 24 shapes not drawn;
- 207 color strings for `colorToRGBA`, valid and not;
- client rects under zero scales and non-scaling strokes;
- z-order, `moveTo`, `remove` and `destroy` sequences;
- event dispatch: bubbling, `cancelBubble`, non-listening ancestors,
  namespaces, the `konva` namespace, and change events.

Measured on 13 November 2026 on Windows with Node 22.18.0
(`cargo test --release -p konva --test oracle -- --nocapture`):

- 4,883 results, 174 events and 8,677 context calls: the same structure,
  strings, booleans, nodes, event order and call order.
- 23,019 of 23,032 numbers are bit-identical. Of the other 13, 12 are
  1 ulp apart and one is listed in
  [tests/fixtures/konva-differences.txt](tests/fixtures/konva-differences.txt).
  All 13 come from one shape rotated by -111.1°, where V8's `Math.sin`
  and fdlibm differ by one ulp; `decompose()`'s skew cancels and turns that
  into 223 ulps.
- `js::hypot` matches Node's `Math.hypot` on 4,000 of 4,000 seeded pairs,
  and `js::number_to_string` matches `String(x)` on 2,000 of 2,000, bit
  for bit and character for character.

Negative checks: letting events bubble past `cancelBubble` fails the test
at the first such dispatch, and scaling shadow blur by the larger scale
instead of the smaller changes 59 numbers and fails it.

## Konva's behavior the port keeps

- A layer's width and height are its stage's; setting them does nothing.
  A layer without a stage has none (`undefined` in Konva, NaN in the port).
- `setZIndex` with an index out of range is clamped as
  `Array.prototype.splice` clamps it.
- `off()` with no argument keeps listeners in the `konva` namespace.
- Adding a layer to the stage draws it, even with automatic drawing off.
- The hit canvas is filled for every shape with `fillEnabled`, whether or
  not it has a fill color.
