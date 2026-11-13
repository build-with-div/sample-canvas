"""Count web platform API uses in published npm package sources.

Usage: python -I tools/npm_platform_scan.py <dir with one subdir per tarball>
Counts are regex occurrences in one representative build per package (see
BUNDLES), so ESM/CJS/UMD copies are not counted several times. It is a
static text scan, not a trace: it shows which APIs a package's code
mentions, not how often they run. Output: JSON. docs/npm-platform-usage.md
records the result and its caveats.
"""
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])

# Representative single build per package, so ESM/CJS/UMD copies are not
# counted several times. Paths are inside each extracted tarball directory.
BUNDLES = {
    # label: (tarball dir, [files, all concatenated] or None for every .js)
    "konva 10.9.1": ("konva-10.9.1", ["package/konva.js"]),
    "fabric 7.4.0": ("fabric-7.4.0", ["package/dist/index.mjs"]),
    "chart.js 4.6.1": ("chart.js-4.6.1", ["package/dist/chart.js"]),
    "roughjs 4.6.6": ("roughjs-4.6.6", ["package/bundled/rough.esm.js"]),
    "pixi.js-legacy 7.4.3": ("pixi.js-legacy-7.4.3", ["package/dist/pixi-legacy.mjs"]),
    "pixi.js 8.23.0": ("pixi.js-8.23.0", ["package/dist/pixi.mjs"]),
    "@excalidraw/excalidraw 0.18.1": ("excalidraw-0.18.1", "package/dist/prod/*.js"),
    "three 0.187.0 (WebGLRenderer)": ("three-0.187.0", ["package/build/three.core.js", "package/build/three.module.js"]),
    "three 0.187.0 (WebGPURenderer)": ("three-0.187.0", ["package/build/three.core.js", "package/build/three.webgpu.js"]),
    "@babylonjs/core 9.36.0": ("core-9.36.0", None),
    "@deck.gl/core 9.4.1": ("core-9.4.1", ["package/dist/index.cjs"]),
    "@luma.gl/webgpu 9.4.2": ("webgpu-9.4.2", ["package/dist/index.cjs"]),
    "@luma.gl/webgl 9.4.2": ("webgl-9.4.2", ["package/dist/index.cjs"]),
}

FEATURES = {
    # Canvas 2D context
    "getContext('2d')": r"getContext\(\s*['\"]2d['\"]",
    "getContext('webgl'/'webgl2')": r"getContext\(\s*['\"](?:webgl2?|experimental-webgl)['\"]",
    "getContext('webgpu')": r"getContext\(\s*['\"]webgpu['\"]",
    "navigator.gpu": r"navigator\.gpu\b",
    "beginPath/moveTo/lineTo": r"\.(?:beginPath|moveTo|lineTo)\(",
    "bezierCurveTo/quadraticCurveTo": r"\.(?:bezierCurveTo|quadraticCurveTo)\(",
    "arc/arcTo/ellipse": r"\.(?:arc|arcTo|ellipse)\(",
    "rect/roundRect": r"\.(?:rect|roundRect)\(",
    "fill/stroke": r"\.(?:fill|stroke)\(",
    "fillRect/strokeRect/clearRect": r"\.(?:fillRect|strokeRect|clearRect)\(",
    "lineWidth/lineCap/lineJoin/miterLimit": r"\.(?:lineWidth|lineCap|lineJoin|miterLimit)\b",
    "setLineDash/lineDashOffset": r"\.(?:setLineDash|getLineDash|lineDashOffset)\b",
    "fillStyle/strokeStyle": r"\.(?:fillStyle|strokeStyle)\b",
    "createLinearGradient/RadialGradient": r"\.create(?:Linear|Radial)Gradient\(",
    "createConicGradient": r"\.createConicGradient\(",
    "createPattern": r"\.createPattern\(",
    "save/restore": r"\.(?:save|restore)\(\)",
    "translate/rotate/scale": r"\.(?:translate|rotate|scale)\(",
    "transform/setTransform/getTransform": r"\.(?:setTransform|getTransform|resetTransform|transform)\(",
    "clip": r"\.clip\(",
    "globalAlpha": r"\.globalAlpha\b",
    "globalCompositeOperation": r"\.globalCompositeOperation\b",
    "shadowBlur/shadowColor/shadowOffset": r"\.shadow(?:Blur|Color|OffsetX|OffsetY)\b",
    "filter (ctx.filter)": r"\.filter\s*=\s*['\"`a-zA-Z]",
    "imageSmoothingEnabled/Quality": r"\.imageSmoothing(?:Enabled|Quality)\b",
    "font": r"\.font\s*=",
    "textAlign/textBaseline/direction": r"\.(?:textAlign|textBaseline)\b",
    "fillText/strokeText": r"\.(?:fillText|strokeText)\(",
    "measureText": r"\.measureText\(",
    "letterSpacing/wordSpacing/fontKerning": r"\.(?:letterSpacing|wordSpacing|fontKerning)\b",
    "drawImage": r"\.drawImage\(",
    "getImageData": r"\.getImageData\(",
    "putImageData": r"\.putImageData\(",
    "createImageData": r"\.createImageData\(",
    "isPointInPath/isPointInStroke": r"\.isPointIn(?:Path|Stroke)\(",
    "Path2D": r"\bnew Path2D\b|\bPath2D\(",
    "willReadFrequently": r"willReadFrequently",
    "toDataURL/toBlob": r"\.(?:toDataURL|toBlob)\(",
    "OffscreenCanvas": r"\bOffscreenCanvas\b",
    "createImageBitmap": r"\bcreateImageBitmap\(",
    "document.createElement('canvas')": r"createElement\(\s*['\"]canvas['\"]",
    # Around the canvas
    "requestAnimationFrame": r"\brequestAnimationFrame\(",
    "devicePixelRatio": r"\bdevicePixelRatio\b",
    "ResizeObserver": r"\bResizeObserver\b",
    "matchMedia (resolution)": r"\bmatchMedia\(",
    "pointer events": r"['\"]pointer(?:down|move|up|cancel|enter|leave|over|out)['\"]",
    "mouse events": r"['\"](?:mousedown|mousemove|mouseup|mouseenter|mouseleave|mouseover|mouseout)['\"]",
    "touch events": r"['\"]touch(?:start|move|end|cancel)['\"]",
    "wheel": r"['\"](?:wheel|mousewheel|DOMMouseScroll)['\"]",
    "dblclick": r"['\"]dblclick['\"]",
    "contextmenu": r"['\"]contextmenu['\"]",
    "setPointerCapture": r"\bsetPointerCapture\(",
    "offsetX/offsetY": r"\.offset[XY]\b",
    "getBoundingClientRect": r"\.getBoundingClientRect\(",
    "keydown/keyup": r"['\"]key(?:down|up)['\"]",
    "FontFace / document.fonts": r"\bnew FontFace\b|document\.fonts\b",
    "Image / HTMLImageElement": r"\bnew Image\(|HTMLImageElement",
    "HTMLVideoElement": r"HTMLVideoElement",
    "WebGL texImage2D": r"\.texImage2D\(",
    "WebGPU configure/getCurrentTexture": r"\.getCurrentTexture\(",
    "WebGPU copyExternalImageToTexture": r"copyExternalImageToTexture",
}

def files_for(pkg, spec):
    if spec is None:
        return sorted(p for p in pkg.rglob("*.js") if not p.name.endswith(".min.js"))
    if isinstance(spec, str):
        return sorted(pkg.glob(spec))
    return [pkg / name for name in spec]

results = {}
for label, (dirname, spec) in BUNDLES.items():
    pkg = root / dirname
    files = files_for(pkg, spec)
    text = chr(10).join(f.read_text(encoding="utf-8", errors="replace") for f in files)
    results[label] = {
        "files": len(files),
        "bytes": len(text),
        "counts": {k: len(re.findall(rx, text)) for k, rx in FEATURES.items()},
    }
json.dump(results, sys.stdout, indent=1)
