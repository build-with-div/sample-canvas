# WPT html/canvas census (estimate)

Taken on 13 November 2026 for [design.md §7](design.md#7-fidelity-plan-for-route-b).
It is a regex classification of each test's code, so the buckets are
estimates, not measurements.

## Inputs

- **Tree listing.** `git ls-tree -r --name-only HEAD
  third_party/blink/web_tests/external/wpt/html/canvas` in a blob-less
  Chromium checkout at `09d377d9438dc95267369f74a073acd81bdde38f` (the
  Chromium 147 tree Div's tools use). It has 4,448 paths.
- **YAML definitions.** The 19 files in `html/canvas/tools/yaml/` at the
  same commit, fetched with:

  ```
  gh api -H "Accept: application/vnd.github.raw"     "repos/chromium/chromium/contents/third_party/blink/web_tests/external/wpt/html/canvas/tools/yaml/<name>.yaml?ref=09d377d9438dc95267369f74a073acd81bdde38f"
  ```

## Command

```
python -I tools/wpt_canvas_census.py <yaml dir> <tree listing>
```

The script needs PyYAML.

## Output

```
element .html tests (excluding refs): 1350
matched to a YAML entry: 1157; unmatched (hand-written): 193

Primary bucket (first blocker in the rule order):
    125  (reftests   0)  js-semantics (exceptions/types/IDL)
     37  (reftests   0)  async/promise/timers
      0  (reftests   0)  toDataURL/toBlob/encoding
    191  (reftests   1)  video/ImageBitmap/OffscreenCanvas source
     18  (reftests  16)  filters (ctx.filter / SVG filters)
     22  (reftests  13)  layers (beginLayer)
     11  (reftests   1)  color space / wide gamut / HDR
     35  (reftests   7)  reset / contextlost / canvas host
     21  (reftests   0)  images (drawImage/createPattern/new Image)
     37  (reftests   0)  pixel readback (get/putImageData)
     90  (reftests  16)  text (fillText/measureText/font)
     73  (reftests   0)  Path2D / DOMMatrix / hit tests
     50  (reftests   0)  shadows
     16  (reftests   0)  compositing / globalAlpha
    431  (reftests   1)  core: paths, fill, stroke, gradients, transforms, clip

Tests touching each feature (not exclusive):
    125  js-semantics (exceptions/types/IDL)
     39  async/promise/timers
      7  toDataURL/toBlob/encoding
    232  video/ImageBitmap/OffscreenCanvas source
     33  filters (ctx.filter / SVG filters)
     48  layers (beginLayer)
     23  color space / wide gamut / HDR
     52  reset / contextlost / canvas host
    245  images (drawImage/createPattern/new Image)
     88  pixel readback (get/putImageData)
    178  text (fillText/measureText/font)
     86  Path2D / DOMMatrix / hit tests
    140  shadows
    186  compositing / globalAlpha

Unmatched directories:
Counter({'manual': 126, 'layers': 34, 'filters': 24, 'text': 4, 'global-hdr-headroom': 3, '2d.text-outside-of-the-flat-tree.html': 1, 'path-objects': 1})
```

## How to read it

- **Primary bucket** is the first rule a test matches, in the order listed.
  The list runs from the hardest blockers to the core, so each test is
  counted once.
- **Unmatched** tests are hand-written pages with no YAML entry. 126 of
  them are in `manual/`.
- **Reftests** are counted from the YAML `reference`/`html_reference`
  keys. The tree also has 137 `-expected.html` files in `element/`.
- **The video/ImageBitmap bucket** includes pattern and `drawImage`
  tests, which take their source from `createImageBitmap` in some
  variants. A Div port feeds them from image resources, so design.md
  counts them under images (M2).
