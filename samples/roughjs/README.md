# roughjs

A port of [rough.js](https://github.com/rough-stuff/rough) 4.6.6 to Rust:

- `RoughGenerator`, which turns lines, rectangles, ellipses, arcs, curves,
  polygons and SVG paths into hand-drawn drawing operations;
- its fill styles;
- the four packages it depends on: `hachure-fill`, `path-data-parser`,
  `points-on-curve` and `points-on-path`;
- `RoughCanvas` (`src/canvas.rs`), the renderer that draws the operations
  with a `CanvasRenderingContext2D`.

The generator is pure geometry and uses no web platform feature. The
renderer needs `<canvas>` (W1) and `CanvasRenderingContext2D` (W2), and Div
has neither yet. Until it does, `RoughCanvas` takes any type implementing
`RoughCanvasContext`, a trait naming the 13 context members rough.js uses
by their web names. Its only implementation is `CallRecorder`, which
records the calls and draws nothing. Both are labeled `// div-gap: W2` and
are deleted when Div's 2D context lands; see
[docs/phase2-plan.md](../../docs/phase2-plan.md).

The port keeps JavaScript's evaluation order and number semantics
(`src/js.rs`), so a seeded drawing matches the original.

## How it is checked

[tests/oracle.rs](tests/oracle.rs) runs the calls in
[tests/fixtures/cases.json](tests/fixtures/cases.json) on the port. It
compares the results with what rough.js 4.6.6 itself returned under
Node 22, recorded in `tests/fixtures/rough-4.6.6.json` by
[tools/rough-oracle](../../tools/rough-oracle). The comparison covers every
op, every coordinate, and the `toPaths` and `opsToPath` strings.

Measured on 13 November 2026 (`cargo test -p roughjs --test oracle -- --nocapture`):

- 92 calls, with identical structure.
- 42,876 of 42,903 numbers are bit-identical. The other 27 are within
  2 ulps; the largest difference is 2.8e-14.
- 525 of 532 strings are identical. The other 7 differ only in those
  numbers' last digits.

The differences come from `Math.sin` and `Math.cos`. The port uses fdlibm,
from the `libm` crate. On 200,000 inputs it matches Node 22's V8 bit for
bit for `tan`, `asin` and `atan`, but differs for about 0.9% of `sin` and
`cos` results. That fits V8 building with its glibc-derived `sin` and
`cos`. The test allows 2 ulps for this reason only. The structure has no
tolerance.

[tests/canvas_oracle.rs](tests/canvas_oracle.rs) does the same for
`RoughCanvas`. The oracle records every access rough.js 4.6.6 makes to its
context through a JavaScript `Proxy`, so a member the port does not expect
would show up in the trace. The cases in
[tests/fixtures/canvas-cases.json](tests/fixtures/canvas-cases.json) cover
each shape, each kind of op set, stroke `none`, line dashes and their
offsets (including an empty dash array and a zero offset),
`fixedDecimalPlaceDigits`, generator config defaults and a path parse error.

Measured on 13 November 2026 (`cargo test -p roughjs --test canvas_oracle -- --nocapture`):

- 19 calls and 811 context accesses, with the same members, in the same
  order, with the same strings. rough.js asks for one `'2d'` context and
  uses only the 13 members the trait names.
- 2,765 of 2,769 numbers are bit-identical. The other 4 are within 2 ulps;
  the largest difference is 3.6e-15. They are in one ellipse, so they come
  from the same `sin` and `cos` difference.

This checks which calls the port makes, not what a context draws with them.

`Math.random` comes from `roughjs::js::math_random`. The test replaces it,
on both sides, with the same seeded generator.

rough.js and its dependencies are MIT licensed; see
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
