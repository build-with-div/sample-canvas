# Which web platform features canvas packages use

Taken 13 November 2026. The packages were downloaded from the npm
registry, then extracted and scanned as text; none of their code was run.
The script is `tools/npm_platform_scan.py`; this page records its output.
[design.md §1](design.md#1-packages-and-the-platform-they-run-on) ranks the
platform work from these numbers.

## Packages and the files scanned

| Column | Package | File(s) |
| --- | --- | --- |
| konva | konva 10.9.1 | `konva.js` |
| fabric | fabric 7.4.0 | `dist/index.mjs` |
| chartjs | chart.js 4.6.1 | `dist/chart.js` |
| rough | roughjs 4.6.6 | `bundled/rough.esm.js` |
| pixi7L | pixi.js-legacy 7.4.3 (the last PixiJS with a Canvas 2D renderer) | `dist/pixi-legacy.mjs` |
| pixi8 | pixi.js 8.23.0 | `dist/pixi.mjs` |
| excal | @excalidraw/excalidraw 0.18.1 | `dist/prod/*.js` (minified; member names survive) |
| 3webgl | three 0.187.0, WebGLRenderer | `build/three.core.js` + `build/three.module.js` |
| 3webgpu | three 0.187.0, WebGPURenderer | `build/three.core.js` + `build/three.webgpu.js` |
| babylon | @babylonjs/core 9.36.0 | all 3,393 `.js` modules |
| deck | @deck.gl/core 9.4.1 | `dist/index.cjs` |
| lumaGPU | @luma.gl/webgpu 9.4.2 | `dist/index.cjs` |
| lumaGL | @luma.gl/webgl 9.4.2 | `dist/index.cjs` |

## Command

Download each tarball with `npm view <pkg> dist.tarball`, then curl it into
its own directory and extract it with `tar -xzf`. Then run:

```
python -I tools/npm_platform_scan.py <dir of extracted tarballs>
```

## Counts

Each number is how often the pattern occurs in the scanned code.

```
feature                                konva  fabric chartjs   rough  pixi7L   pixi8   excal  3webgl 3webgpu babylon    deck lumaGPU  lumaGL
getContext('2d')                           5      21       2       1      19      25      12       5       2      16       0       1       1
getContext('webgl'/'webgl2')               0       3       0       0      10       6       0       1       1       9       0       0       3
getContext('webgpu')                       0       0       0       0       0       2       0       0       1       1       0       2       0
navigator.gpu                              0       0       0       0       0       2       0       0       2       5       0       6       0
beginPath/moveTo/lineTo                   91      59      64       3      61      72      62      14      14       9       0       0       0
bezierCurveTo/quadraticCurveTo             8       7       4       1      24      16       4       9       9       2       0       0       0
arc/arcTo/ellipse                         22       5      18       6       5      16       6       0       0       6       0       0       0
rect/roundRect                            16       0       4       0       2      19       8       0       0       0       0       0       0
fill/stroke                               33      13      22       2      11      59      32       2       3      78       6       0       4
fillRect/strokeRect/clearRect              8      22       6       0      14      16      21       0       0      17       0       0       1
lineWidth/lineCap/lineJoin/miterLimi      49      15      24       2      26      32      33       2       3       7       0       0       2
setLineDash/lineDashOffset                 4       5      15       4       0       0       8       0       0       0       0       0       0
fillStyle/strokeStyle                     11      43      40      10      36      49      91       0       0      22       0       0       0
createLinearGradient/RadialGradient        5       2       0       0       4       5       0       0       0       1       0       0       0
createConicGradient                        0       0       0       0       0       0       0       0       0       0       0       0       0
createPattern                              2       4       0       0       2       4       0       0       0       0       0       0       0
save/restore                              64      73      34       6      18      41      68       0       0       2       0       0       0
translate/rotate/scale                    50      40       3       0      14     116      59       7       7     202      22       0       0
transform/setTransform/getTransform       24      66       0       0       7      24       1       0       0      10       4       0       0
clip                                       3       0       6       0       5       3       2       0       0       0       0       0       0
globalAlpha                                1       7       1       0      23      13       4       0       0       0       0       0       0
globalCompositeOperation                   4       6       0       0       9      11       0       0       0       0       0       0       0
shadowBlur/shadowColor/shadowOffset       88      17       0       0      16      20       0       0       0       0       3       0       0
filter (ctx.filter)                        0       0       0       0       0       4       3       3       3      14       0       0       0
imageSmoothingEnabled/Quality             24       4       0       0       1       3       3       0       0       0       0       0       0
font                                       4       2      11       0       6      12       4       0       0       4       0       0       0
textAlign/textBaseline/direction           4      12      29       0       7      11      22       0       0       1       0       0       0
fillText/strokeText                        8       2       3       0       7       8       2       0       0       1       0       0       0
measureText                                5       3       6       0      11      20       4       0       0       3       0       0       0
letterSpacing/wordSpacing/fontKernin       9       0       0       0       9      20       0       0       0       0       0       0       0
drawImage                                 14      10       0       0      15      22      11       3       2      20       0       1       1
getImageData                               8       5       0       0       7       8       1       1       1       7       0       1       0
putImageData                               3       3       0       0       4       4       0       2       2       2       0       1       0
createImageData                            2       4       0       0       0       2       0       0       0       1       0       0       0
isPointInPath/isPointInStroke              2       0       0       0       0       0       0       0       0       0       0       0       0
Path2D                                     1       0       1       0       0       0       1       0       0       2       0       0       0
willReadFrequently                         8       2       0       0       3       5       0       0       0       0       0       0       0
toDataURL/toBlob                           6       8       1       0       5       5       6       1       1      33       0       0       0
OffscreenCanvas                            0       0       2       0       5      11       0      10       9       8       2       2       1
createImageBitmap                          0       0       0       0       5       6       0       2       2      14       0       0       0
document.createElement('canvas')           1       1       0       0       2       4      13       0       0      12       1       1       0
requestAnimationFrame                      2       1       0       0       4       5      10       4      12      12       2       0       1
devicePixelRatio                           3       4       4       0       0       1      49       1       1      13      14       0       0
ResizeObserver                             0       0       1       0       0       4       5       0       0       2       0       0       0
matchMedia (resolution)                    0       0       0       0       0       0       0       0       0       1       0       0       0
pointer events                            10       0       0       0      38      45      43       0       0      31       1       0       0
mouse events                              19       6      17       0      32      33       0       0       0       6       0       0       0
touch events                              15      11       0       0      12      12       8       0       0       0       0       0       0
wheel                                      1       2       0       0       3       3       3       0       0      20       2       0       0
dblclick                                   1       3       0       0       0       0       0       0       0       0       6       0       0
contextmenu                                1       4       0       0       0       4       0       0       0       7       0       0       0
setPointerCapture                          7       0       0       0       0       0       1       0       0       5       0       0       0
offsetX/offsetY                           60      39       8       0       2       5      10       8       8      14       0       0       0
getBoundingClientRect                      1       2       0       0       3       2      11       0       0      11       2       0       0
keydown/keyup                              0       0       0       0       4       2      30       0       0      10       2       0       0
FontFace / document.fonts                  0       0       0       0       6       2       8       0       0       0       0       0       0
Image / HTMLImageElement                   5       5       0       0      19       6       1      15      15      36       0       0       0
HTMLVideoElement                           0       0       0       0      10       2       0       4       7       2       0       0       0
WebGL texImage2D                           0       2       0       0       7      13       0      16       2      27       0       0       0
WebGPU configure/getCurrentTexture         0       0       0       0       0       3       0       0       3       1       0       2       0
WebGPU copyExternalImageToTexture          0       0       0       0       0       1       0       0       4       3       0       1       0
files                                      1       1       1       1       1       1       9       2       2    3393       1       1       1
```

## Caveats

- **Static mentions, not runtime frequency.** A count says the package's
  code calls a member; a trace measures how often it runs. The charts
  session plans a call trace for Chart.js and ECharts; see design.md §7.
- **Some patterns match other things.** `offsetX/offsetY` also matches
  Konva's own node `offsetX` property, and `translate/rotate/scale` matches
  matrix helpers in Babylon and deck.gl.
- **Event names are counted as string literals,** so an event that a
  package listens to through a lookup table can be missed.

## Specifics checked by hand

- **Konva hit-tests with a second canvas.** It draws every shape into a
  hidden "hit canvas" in a unique color key and reads that canvas back with
  `getImageData` (`hitCanvas` occurs 38 times, `colorKey` 9 times,
  `willReadFrequently` 8 times). So readback exactness and anti-aliasing
  control are part of input handling, not only export.
- **Fabric layers two canvases.** It stacks `lowerCanvasEl` and
  `upperCanvasEl` and checks transparency per pixel for hit testing
  (`perPixelTargetFind`, `isTargetTransparent`).
- **three.js's `WebGPURenderer` falls back to WebGL2.** It contains
  `class WebGLBackend` for that fallback. It configures the canvas context
  with `alphaMode` "premultiplied" when `alpha` is set, and calls
  `navigator.gpu.requestAdapter`.
- **luma.gl's WebGPU device** gets its adapter the same way
  (`requestAdapter(requestAdapterOptions)`).
