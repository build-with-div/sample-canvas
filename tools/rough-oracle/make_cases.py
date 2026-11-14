"""Writes samples/roughjs/tests/fixtures/cases.json, the inputs that
oracle.cjs runs through rough.js and samples/roughjs/tests/oracle.rs runs
through the port.

    python -I tools/rough-oracle/make_cases.py
"""
import json
import math
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "samples/roughjs/tests/fixtures/cases.json"

FILL_STYLES = ["hachure", "solid", "zigzag", "cross-hatch", "dots", "dashed", "zigzag-line"]


def call(method, *args, options=None):
    return {"method": method, "args": list(args), "options": options}


def seeded():
    """Every shape and option family, each call seeded."""
    calls = []
    seed = 1

    def opts(**kw):
        nonlocal seed
        seed += 1
        return {"seed": seed, **kw}

    # Lines: under 200, between 200 and 500, over 500 (three roughness gains).
    for x2, y2 in [(200, 100), (310, 220), (700, 400)]:
        calls.append(call("line", 10, 10, x2, y2, options=opts()))
    calls.append(call("line", 10, 10, 12, 11, options=opts()))  # shorter than the offset
    calls.append(call("line", 10, 10, 200, 100, options=opts(roughness=0)))
    calls.append(call("line", 10, 10, 200, 100, options=opts(bowing=6, roughness=2.5)))
    calls.append(call("line", 10, 10, 200, 100, options=opts(preserveVertices=True)))
    calls.append(call("line", 10, 10, 200, 100, options=opts(disableMultiStroke=True)))
    calls.append(call("line", 10, 10, 200, 100, options=opts(maxRandomnessOffset=0)))

    # Rectangles with every fill style, and option variants.
    calls.append(call("rectangle", 10, 10, 200, 100, options=opts()))
    # The dots, dashed and zigzag-line fills draw many small shapes, so they
    # get a smaller rectangle and a wider gap to keep the oracle file small.
    heavy = {"dots", "dashed", "zigzag-line"}
    for style in FILL_STYLES:
        if style in heavy:
            calls.append(call("rectangle", 10, 10, 60, 40, options=opts(fill="red", fillStyle=style, hachureGap=10)))
        else:
            calls.append(call("rectangle", 10, 10, 200, 100, options=opts(fill="red", fillStyle=style)))
    for extra in [
        dict(hachureAngle=0),
        dict(hachureAngle=60, hachureGap=8),
        dict(hachureGap=0.05),
        dict(strokeWidth=3, fillWeight=2),
        dict(roughness=0),
        dict(roughness=0.5),
        dict(disableMultiStrokeFill=True),
        dict(stroke="none"),
        dict(fillStyle="dashed", dashOffset=5, dashGap=3, hachureGap=12),
        dict(fillStyle="zigzag-line", zigzagOffset=4, hachureGap=12),
        dict(fillStyle="unknown-style"),
    ]:
        calls.append(call("rectangle", 20, 30, 160, 90, options=opts(fill="#0a0", **extra)))

    # Ellipses and circles.
    calls.append(call("ellipse", 150, 100, 200, 120, options=opts()))
    for style in ["hachure", "solid", "cross-hatch", "zigzag"]:
        calls.append(call("ellipse", 150, 100, 200, 120, options=opts(fill="blue", fillStyle=style)))
    calls.append(call("ellipse", 50, 50, 60, 40, options=opts(fill="blue", fillStyle="dots", hachureGap=10)))
    calls.append(call("ellipse", 150, 100, 200, 120, options=opts(roughness=0)))
    calls.append(call("ellipse", 150, 100, 200, 120, options=opts(roughness=0, fill="blue")))
    calls.append(call("ellipse", 150, 100, 400, 60, options=opts(roughness=2.5, curveFitting=0.5)))
    calls.append(call("ellipse", 150, 100, 80, 80, options=opts(curveStepCount=20, disableMultiStroke=True)))
    calls.append(call("circle", 80, 80, 100, options=opts(fill="red")))

    # Paths of points.
    zigzag = [[10, 10], [60, 80], [110, 20], [160, 90], [210, 30]]
    calls.append(call("linearPath", zigzag, options=opts()))
    calls.append(call("linearPath", [[0, 0], [50, 50]], options=opts()))
    calls.append(call("linearPath", [[0, 0]], options=opts()))
    star = [[100, 10], [125, 80], [195, 80], [140, 120], [160, 190], [100, 150], [40, 190], [60, 120], [5, 80], [75, 80]]
    calls.append(call("polygon", star, options=opts()))
    for style in ["hachure", "solid", "zigzag", "cross-hatch"]:
        calls.append(call("polygon", star, options=opts(fill="purple", fillStyle=style)))

    # Arcs.
    for start, stop, closed in [
        (-math.pi / 2, math.pi * 0.75, False),
        (-math.pi / 2, math.pi * 0.75, True),
        (0.3, 0.3 + 3 * math.pi, False),
        (math.pi, 1.5 * math.pi, True),
    ]:
        calls.append(call("arc", 150, 150, 200, 160, start, stop, closed, options=opts()))
    calls.append(call("arc", 150, 150, 200, 160, -math.pi / 2, math.pi, True, options=opts(fill="red")))
    calls.append(call("arc", 150, 150, 200, 160, -math.pi / 2, math.pi, True, options=opts(fill="red", fillStyle="solid")))

    # Curves.
    wave = [[10, 50], [60, 10], [110, 90], [160, 20], [210, 80], [260, 40]]
    calls.append(call("curve", [[10, 10], [100, 100]], options=opts()))
    calls.append(call("curve", [[10, 10], [60, 90], [120, 20]], options=opts()))
    calls.append(call("curve", wave, options=opts()))
    calls.append(call("curve", wave, options=opts(curveTightness=0.5, roughness=2)))
    calls.append(call("curve", [wave, [[20, 120], [80, 160], [140, 110]]], options=opts()))
    for style in ["hachure", "solid"]:
        calls.append(call("curve", wave, options=opts(fill="orange", fillStyle=style)))
        calls.append(call("curve", [[10, 10], [60, 90], [120, 20]], options=opts(fill="orange", fillStyle=style)))

    # SVG paths.
    paths = [
        "M10 10 L 100 10 L 100 100 Z",
        "m10 10 l90 0 l0 90 z",
        "M10 80 C 40 10, 65 10, 95 80 S 150 150, 180 80",
        "M10 80 Q 52.5 10, 95 80 T 180 80",
        "M10 10 H 90 V 90 H 10 Z",
        "M80 80 A 45 45, 0, 0, 0, 125 125 L 125 80 Z",
        "M80 80 A 45 45, 0, 1, 1, 125 125",
        "M10 10 a 30 50 30 1 0 60 20",
        "M10 10 h 50 v 50 h -50 z M100 10 h 50 v 50 h -50 z",
        "M10,10L- 50,50 L\n90,10",
        "L 50 50 L 90 10",
        "M10 10 L 50 50 # 2",
    ]
    for d in paths:
        calls.append(call("path", d, options=opts()))
    for style in ["hachure", "solid", "cross-hatch"]:
        calls.append(call("path", paths[0], options=opts(fill="teal", fillStyle=style)))
        calls.append(call("path", paths[8], options=opts(fill="teal", fillStyle=style)))
    calls.append(call("path", paths[2], options=opts(fill="teal", fillStyle="solid")))
    calls.append(call("path", paths[2], options=opts(simplification=0.5)))
    calls.append(call("path", paths[5], options=opts(fill="transparent")))
    calls.append(call("path", "", options=opts()))
    return calls


def groups():
    return [
        {"name": "seeded", "mathRandomSeed": 1, "config": None, "calls": seeded()},
        {
            # No seed: rough.js uses Math.random, and calls without options
            # share the generator's default options and their randomizer.
            "name": "unseeded-shared-defaults",
            "mathRandomSeed": 20261113,
            "config": None,
            "calls": [
                call("rectangle", 10, 10, 100, 50),
                call("ellipse", 100, 100, 80, 40),
                call("rectangle", 10, 10, 100, 50, options={"fill": "red"}),
                call("line", 0, 0, 50, 50),
            ],
        },
        {
            # A seed in the generator's config: calls without options keep
            # drawing from one randomizer; calls with options start from a
            # copy that shares it.
            "name": "config-seed",
            "mathRandomSeed": 7,
            "config": {"options": {"seed": 7, "roughness": 2}},
            "calls": [
                call("line", 0, 0, 100, 100),
                call("line", 0, 0, 100, 100),
                call("rectangle", 10, 10, 100, 50, options={"fill": "red", "fillStyle": "cross-hatch"}),
                call("circle", 50, 50, 60),
            ],
        },
    ]


OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(json.dumps({"groups": groups()}, indent=1) + "\n", encoding="utf-8", newline="\n")
print(f"wrote {OUT}")
