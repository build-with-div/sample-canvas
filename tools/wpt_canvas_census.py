"""Classify WPT html/canvas/element tests by what a Div 2D context port needs.

Inputs: the canvas YAML test definitions (html/canvas/tools/yaml/*.yaml) and
a listing of the html/canvas tree (one repository path per line), both from
the Chromium 147 tree. Output: counts per blocker bucket, weighted by the
generated element/ .html files of each YAML entry. This is a regex estimate,
not a measurement; see docs/wpt-canvas-census.md for how to reproduce it.

    python -I tools/wpt_canvas_census.py <yaml dir> <tree listing>
"""
import collections
import json
import re
import sys
from pathlib import Path

import yaml

yaml_dir = Path(sys.argv[1])
tree_list = Path(sys.argv[2])
prefix = "third_party/blink/web_tests/external/wpt/html/canvas/"

entries = {}
for path in sorted(yaml_dir.glob("*.yaml")):
    for item in yaml.safe_load(path.read_text(encoding="utf-8")) or []:
        if isinstance(item, dict) and "name" in item:
            entries[item["name"]] = (path.stem, item)

names = sorted(entries, key=len, reverse=True)
files = [l.strip()[len(prefix):] for l in tree_list.read_text().splitlines()]
element_html = [f for f in files if f.startswith("element/") and f.endswith(".html")
                and not f.endswith("-expected.html") and not f.endswith("-ref.html")]

def entry_for(base):
    for n in names:
        if base == n or base.startswith(n + ".") or base.startswith(n + "_") or base.startswith(n):
            return n
    return None

RULES = [
    # (bucket, regex over code+attributes). First match wins for "primary".
    ("js-semantics (exceptions/types/IDL)", r"@assert throws|assert_throws|typeof|instanceof|Object\.|prototype|@assert .* === undefined|\bnew Proxy|\.length\b|toString|hasOwnProperty|in window"),
    ("async/promise/timers", r"\bawait\b|new Promise|step_timeout|requestAnimationFrame|\.onload\b|createImageBitmap"),
    ("toDataURL/toBlob/encoding", r"toDataURL|toBlob|convertToBlob"),
    ("video/ImageBitmap/OffscreenCanvas source", r"createImageBitmap|ImageBitmap|<video|video|transferToImageBitmap|OffscreenCanvas|VideoFrame"),
    ("filters (ctx.filter / SVG filters)", r"\.filter\s*=|CanvasFilter|feGaussian|filter:"),
    ("layers (beginLayer)", r"beginLayer|endLayer"),
    ("color space / wide gamut / HDR", r"colorSpace|display-p3|rec2020|float16|colorType|HdrHeadroom"),
    ("reset / contextlost / canvas host", r"\.reset\(|contextlost|contextrestored|isContextLost"),
    ("images (drawImage/createPattern/new Image)", r"drawImage|createPattern|new Image|\.png|\.jpg|\.svg|\.gif"),
    ("pixel readback (get/putImageData)", r"getImageData|putImageData|createImageData"),
    ("text (fillText/measureText/font)", r"fillText|strokeText|measureText|\.font\b|letterSpacing|wordSpacing|fontKerning|textRendering|direction|textAlign|textBaseline"),
    ("Path2D / DOMMatrix / hit tests", r"Path2D|DOMMatrix|getTransform|isPointInPath|isPointInStroke|roundRect"),
    ("shadows", r"shadow"),
    ("compositing / globalAlpha", r"globalCompositeOperation|globalAlpha"),
    ("core: paths, fill, stroke, gradients, transforms, clip", r"."),
]

primary = collections.Counter()
needs = collections.Counter()
reftest = collections.Counter()
unmatched = []
per_file = []
for f in element_html:
    base = Path(f).stem
    n = entry_for(base)
    if n is None:
        unmatched.append(f)
        continue
    stem, item = entries[n]
    # Keys that name canvas types or test metadata would match the rules
    # spuriously (e.g. "OffscreenCanvas" in canvas_types).
    ignored = (
        "canvas_types", "name", "desc", "expected", "test_type",
        "variants_layout", "append_variants_to_name", "canvas",
    )
    text = json.dumps({k: v for k, v in item.items() if k not in ignored})
    hit = None
    for bucket, rx in RULES:
        # Variants await font/image loading as boilerplate; only awaits in
        # the test's own code make it an async test.
        haystack = str(item.get("code", "")) if bucket.startswith("async") else text
        if re.search(rx, haystack):
            needs[bucket] += 1
            if hit is None:
                hit = bucket
    primary[hit] += 1
    if "reference" in item or "html_reference" in item:
        reftest[hit] += 1
    per_file.append((f, hit))

print(f"element .html tests (excluding refs): {len(element_html)}")
print(f"matched to a YAML entry: {len(per_file)}; unmatched (hand-written): {len(unmatched)}")
print("\nPrimary bucket (first blocker in the rule order):")
for bucket, _ in RULES:
    print(f"  {primary[bucket]:5d}  (reftests {reftest[bucket]:3d})  {bucket}")
print("\nTests touching each feature (not exclusive):")
for bucket, _ in RULES[:-1]:
    print(f"  {needs[bucket]:5d}  {bucket}")
print("\nUnmatched directories:")
print(collections.Counter(u.split('/')[1] for u in unmatched))
