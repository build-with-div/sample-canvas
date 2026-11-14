"""Writes samples/konva/tests/fixtures/cases.json: scripts of Konva calls
that tools/konva-oracle/oracle.mjs runs on Konva 10.9.1 and
samples/konva/tests/oracle.rs runs on the port.

    python -I tools/konva-oracle/make_cases.py

Each group is a list of ops, run on a fresh set of nodes. Nodes are named
by handles in the script ("s", "l1", "g3", ...), not by Konva ids.

    ["new", h, class, attrs]       new Konva[class](attrs)
    ["add", parent, [h, ...]]      parent.add(...)
    ["set", h, attrs]              h.setAttrs(attrs)

Attributes are written as [key, value] pairs, because Konva keeps them in
the order they were first set.
    ["call", h, method, arg?]      h[method](arg); the result if any
    ["on", h, events, label, cancelBubble]
    ["off", h, events or null]
    ["fire", h, type, bubble]
    ["get", h, what, arg?]         a query; see QUERIES
    ["tf", [[op, ...args], ...]]   a standalone Konva.Transform

The random choices are seeded, so the file is reproducible.
"""

import json
import os
import random

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
OUT = os.path.join(ROOT, "samples", "konva", "tests", "fixtures", "cases.json")

QUERIES = [
    "transform",
    "absoluteTransform",
    "absolutePosition",
    "clientRect",
    "absoluteScale",
    "absoluteRotation",
    "absoluteOpacity",
    "isVisible",
    "isListening",
    "zIndex",
    "absoluteZIndex",
    "depth",
    "attrs",
    "children",
    "parent",
    "layer",
    "stage",
    "position",
    "size",
    "decompose",
]


def num(rng, lo, hi, digits=1):
    return round(rng.uniform(lo, hi), digits)


def transform_attrs(rng, heavy=True):
    a = {}
    if rng.random() < 0.8:
        a["x"] = num(rng, -150, 150)
    if rng.random() < 0.8:
        a["y"] = num(rng, -150, 150)
    if rng.random() < 0.6:
        a["rotation"] = rng.choice([num(rng, -180, 180), 90, 45, -30, 180])
    if rng.random() < 0.5:
        a["scaleX"] = rng.choice([0.5, 2, -1, 1.5, 0.75, -0.5, num(rng, 0.2, 3, 2)])
    if rng.random() < 0.5:
        a["scaleY"] = rng.choice([0.5, 2, -1, 1.5, 0.75, -0.5, num(rng, 0.2, 3, 2)])
    if heavy and rng.random() < 0.3:
        a["skewX"] = num(rng, -0.6, 0.6, 2)
    if heavy and rng.random() < 0.3:
        a["skewY"] = num(rng, -0.6, 0.6, 2)
    if rng.random() < 0.3:
        a["offsetX"] = num(rng, -40, 40)
    if rng.random() < 0.3:
        a["offsetY"] = num(rng, -40, 40)
    return a


def shape_attrs(rng, cls):
    a = transform_attrs(rng)
    if cls == "Rect":
        a["width"] = rng.choice([num(rng, 5, 120), num(rng, 5, 120), -num(rng, 5, 60), 0])
        a["height"] = rng.choice([num(rng, 5, 120), num(rng, 5, 120), -num(rng, 5, 60)])
    else:
        a["radius"] = rng.choice([num(rng, 3, 60), num(rng, 3, 60), -num(rng, 3, 20)])
    if rng.random() < 0.6:
        a["stroke"] = rng.choice(["black", "#336699", ""])
    if rng.random() < 0.6:
        a["strokeWidth"] = rng.choice([0, 1, 3, 4.5, 10])
    if rng.random() < 0.25:
        a["strokeScaleEnabled"] = False
    if rng.random() < 0.15:
        a["strokeEnabled"] = False
    if rng.random() < 0.3:
        a["shadowColor"] = "black"
        a["shadowBlur"] = rng.choice([0, 5, 12.5])
        a["shadowOffsetX"] = num(rng, -10, 10)
        a["shadowOffsetY"] = num(rng, -10, 10)
        if rng.random() < 0.2:
            a["shadowOpacity"] = 0
        if rng.random() < 0.2:
            a["shadowEnabled"] = False
    if rng.random() < 0.15:
        a["visible"] = False
    if rng.random() < 0.15:
        a["listening"] = False
    if rng.random() < 0.3:
        a["opacity"] = rng.choice([0.5, 0.25, 0.8])
    if rng.random() < 0.1:
        a["transformsEnabled"] = rng.choice(["position", "none"])
    return a


def container_attrs(rng):
    a = transform_attrs(rng)
    if rng.random() < 0.1:
        a["visible"] = False
    if rng.random() < 0.1:
        a["listening"] = False
    if rng.random() < 0.2:
        a["opacity"] = rng.choice([0.5, 0.9])
    if rng.random() < 0.1:
        a["transformsEnabled"] = rng.choice(["position", "none"])
    return a


def query_all(rng, ops, nodes, parents):
    """Every query on every node, with ancestors as `top` and `relativeTo`."""
    for h in nodes:
        ancestors = []
        p = parents.get(h)
        while p:
            ancestors.append(p)
            p = parents.get(p)
        top = rng.choice(ancestors) if ancestors else None
        ops.append(["get", h, "transform"])
        ops.append(["get", h, "absoluteTransform", None])
        if top:
            ops.append(["get", h, "absoluteTransform", top])
        ops.append(["get", h, "absolutePosition", None])
        if top:
            ops.append(["get", h, "absolutePosition", top])
        for config in [{}, {"skipTransform": True}, {"skipStroke": True}, {"skipShadow": True}]:
            ops.append(["get", h, "clientRect", config])
        if top:
            ops.append(["get", h, "clientRect", {"relativeTo": top}])
            ops.append(["get", h, "clientRect", {"relativeTo": top, "skipStroke": True}])
        for what in [
            "absoluteScale",
            "absoluteRotation",
            "decompose",
            "absoluteOpacity",
            "isVisible",
            "isListening",
            "zIndex",
            "absoluteZIndex",
            "depth",
            "position",
            "size",
            "attrs",
        ]:
            ops.append(["get", h, what])


def random_tree(rng):
    ops = []
    nodes = []
    parents = {}
    stage_attrs = {"width": 800, "height": 600}
    if rng.random() < 0.5:
        stage_attrs.update(transform_attrs(rng, heavy=False))
    ops.append(["new", "s", "Stage", stage_attrs])
    nodes.append("s")
    counter = [0]

    def handle(prefix):
        counter[0] += 1
        return f"{prefix}{counter[0]}"

    def fill(parent, depth):
        children = []
        for _ in range(rng.randint(1, 3)):
            if depth < 3 and rng.random() < 0.45:
                h = handle("g")
                ops.append(["new", h, "Group", container_attrs(rng)])
                children.append(h)
                parents[h] = parent
                nodes.append(h)
                fill(h, depth + 1)
            else:
                cls = rng.choice(["Rect", "Circle"])
                h = handle("r" if cls == "Rect" else "c")
                ops.append(["new", h, cls, shape_attrs(rng, cls)])
                children.append(h)
                parents[h] = parent
                nodes.append(h)
        if rng.random() < 0.15:
            h = handle("g")
            ops.append(["new", h, "Group", {}])
            children.append(h)
            parents[h] = parent
            nodes.append(h)
        ops.append(["add", parent, children])

    layers = []
    for _ in range(rng.randint(1, 2)):
        h = handle("l")
        ops.append(["new", h, "Layer", container_attrs(rng)])
        layers.append(h)
        parents[h] = "s"
        nodes.append(h)
        fill(h, 1)
    ops.append(["add", "s", layers])
    query_all(rng, ops, nodes, parents)

    # Change some attributes, with change listeners, then query again.
    for h in rng.sample(nodes, min(3, len(nodes))):
        ops.append(["on", h, "xChange yChange rotationChange scaleXChange", f"change:{h}", False])
    for _ in range(6):
        h = rng.choice(nodes)
        attrs = transform_attrs(rng)
        if rng.random() < 0.3 and "x" in attrs:
            attrs["x"] = attrs["x"]  # the same value twice fires once
        ops.append(["set", h, attrs])
        ops.append(["set", h, {k: v for k, v in attrs.items() if k == "x"}])
    for h in rng.sample(nodes, min(4, len(nodes))):
        ops.append(["call", h, "setAbsolutePosition", {"x": num(rng, -100, 300), "y": num(rng, -100, 300)}])
        ops.append(["get", h, "attrs"])
        ops.append(["get", h, "absolutePosition", None])
    query_all(rng, ops, nodes, parents)
    return ops


def transform_cases(rng):
    ops = []
    for _ in range(60):
        seq = []
        for _ in range(rng.randint(2, 8)):
            kind = rng.choice(["translate", "scale", "rotate", "skew", "multiply", "invert", "point", "decompose", "rect", "isInvertible"])
            if kind == "translate":
                seq.append([kind, num(rng, -100, 100, 2), num(rng, -100, 100, 2)])
            elif kind == "scale":
                seq.append([kind, rng.choice([num(rng, -3, 3, 2), 0, 1, -1]), num(rng, -3, 3, 2)])
            elif kind == "rotate":
                seq.append([kind, rng.choice([num(rng, -7, 7, 3), 0, 3.141592653589793 / 2])])
            elif kind == "skew":
                seq.append([kind, num(rng, -1, 1, 2), num(rng, -1, 1, 2)])
            elif kind == "multiply":
                seq.append([kind, [num(rng, -3, 3, 2) for _ in range(4)] + [num(rng, -50, 50, 1) for _ in range(2)]])
            elif kind in ("point",):
                seq.append([kind, num(rng, -100, 100, 2), num(rng, -100, 100, 2)])
            elif kind == "rect":
                seq.append([kind, num(rng, -50, 50), num(rng, -50, 50), num(rng, -20, 80), num(rng, -20, 80)])
            else:
                seq.append([kind])
        ops.append(["tf", seq])
    # Degenerate matrices for decompose.
    for m in ([0, 0, 0, 0, 5, 6], [0, 0, 2, 3, 1, 1], [0, 0, -2, -3, 0, 0], [2, 0, 4, 0, 0, 0], [1, 1, 1, 1, 0, 0], [-1, 0, 0, 1, 0, 0], [0, -1, 1, 0, 0, 0]):
        ops.append(["tf", [["multiply", m], ["decompose"], ["isInvertible"], ["invert"]]])
    return ops


def collapsed_cases():
    ops = [
        ["new", "s", "Stage", {"width": 400, "height": 300}],
        ["new", "l", "Layer", {"x": 10, "y": 20, "scaleX": 2}],
        ["new", "g", "Group", {"x": 30, "scaleY": 0}],
        ["new", "g2", "Group", {"rotation": 30, "scaleX": 3}],
        ["new", "r1", "Rect", {"x": 5, "width": 40, "height": 20, "stroke": "red", "strokeWidth": 6, "strokeScaleEnabled": False}],
        ["new", "r2", "Rect", {"y": 5, "width": 40, "height": 20, "stroke": "red", "strokeWidth": 6}],
        ["new", "c1", "Circle", {"x": 50, "radius": 10, "stroke": "blue", "strokeWidth": 3, "strokeScaleEnabled": False}],
        ["new", "r3", "Rect", {"width": 10, "height": 10, "scaleX": 0, "stroke": "red", "strokeWidth": 4, "strokeScaleEnabled": False}],
        ["add", "s", ["l"]],
        ["add", "l", ["g", "g2"]],
        ["add", "g", ["r1", "r2"]],
        ["add", "g2", ["c1", "r3"]],
    ]
    for h, tops in [("r1", ["g", "l", "s"]), ("r2", ["g", "l"]), ("c1", ["g2", "l"]), ("r3", ["g2", "l", "s"]), ("g", ["l", "s"]), ("g2", ["l"]), ("l", ["s"])]:
        ops.append(["get", h, "clientRect", {}])
        ops.append(["get", h, "clientRect", {"skipTransform": True}])
        for top in tops:
            ops.append(["get", h, "clientRect", {"relativeTo": top}])
        ops.append(["get", h, "absoluteScale"])
        ops.append(["get", h, "decompose"])
    ops.append(["get", "s", "clientRect", {}])
    return ops


def tree_ops_cases():
    ops = [
        ["new", "s", "Stage", {"width": 400, "height": 300}],
        ["new", "l1", "Layer", {}],
        ["new", "l2", "Layer", {"y": 10}],
        ["new", "g1", "Group", {"x": 10}],
        ["new", "g2", "Group", {"x": 20}],
        ["on", "l1", "add", "add:l1", False],
        ["on", "g1", "add", "add:g1", False],
        ["on", "g2", "add", "add:g2", False],
    ]
    shapes = []
    for i in range(6):
        h = f"r{i}"
        ops.append(["new", h, "Rect", {"x": i * 10, "width": 5, "height": 5}])
        ops.append(["on", h, "destroy", f"destroy:{h}", False])
        shapes.append(h)
    ops += [
        ["add", "s", ["l1", "l2"]],
        ["add", "l1", ["g1", "g2"]],
        ["add", "g1", shapes[:4]],
        ["add", "g2", shapes[4:]],
        ["get", "g1", "children"],
    ]
    for h, method, arg in [
        ("r0", "moveToTop", None),
        ("r0", "moveToTop", None),
        ("r3", "moveDown", None),
        ("r1", "moveUp", None),
        ("r2", "moveToBottom", None),
        ("r2", "moveToBottom", None),
        ("r1", "setZIndex", 2),
        ("r3", "setZIndex", 0),
        ("r3", "setZIndex", 9),
        ("r4", "moveTo", "g1"),
        ("r4", "moveTo", "g1"),
        ("r5", "moveDown", None),
        ("g2", "moveToTop", None),
        ("r0", "remove", None),
        ("r0", "moveTo", "g2"),
        ("g1", "moveTo", "l2"),
    ]:
        ops.append(["call", h, method, arg])
        ops.append(["get", "g1", "children"])
        ops.append(["get", "g2", "children"])
        for x in shapes:
            ops.append(["get", x, "zIndex"])
            ops.append(["get", x, "absoluteZIndex"])
            ops.append(["get", x, "depth"])
            ops.append(["get", x, "parent"])
    ops += [
        ["set", "r1", {"zIndex": 0}],
        ["get", "g1", "children"],
        ["get", "r1", "layer"],
        ["get", "r1", "stage"],
        ["get", "l2", "layer"],
        ["get", "s", "layer"],
        ["get", "s", "stage"],
        ["call", "g1", "destroy", None],
        ["get", "l2", "children"],
        ["get", "r1", "parent"],
        ["get", "r1", "stage"],
        ["call", "r5", "destroy", None],
        ["get", "g2", "children"],
        ["call", "g2", "hide", None],
        ["get", "r0", "isVisible"],
        ["call", "g2", "show", None],
        ["get", "r0", "isVisible"],
    ]
    return ops


def events_cases():
    ops = [
        ["new", "s", "Stage", {"width": 400, "height": 300}],
        ["new", "l", "Layer", {}],
        ["new", "g", "Group", {}],
        ["new", "g2", "Group", {}],
        ["new", "r", "Rect", {"width": 10, "height": 10}],
        ["new", "c", "Circle", {"radius": 4}],
        ["add", "s", ["l"]],
        ["add", "l", ["g"]],
        ["add", "g", ["g2", "c"]],
        ["add", "g2", ["r"]],
    ]
    for h in ["s", "l", "g", "g2", "r", "c"]:
        ops.append(["on", h, "click tap.ns", f"{h}", False])
    ops += [
        ["fire", "r", "click", True],
        ["fire", "r", "click", False],
        ["fire", "g2", "click", True],
        ["fire", "r", "tap", True],
        ["fire", "r", "mouseenter", True],
        ["on", "g", "mouseenter", "g-enter", False],
        ["fire", "r", "mouseenter", True],
        ["fire", "s", "mouseenter", True],
        ["on", "g2", "click", "g2-stop", True],
        ["fire", "r", "click", True],
        ["off", "g2", "click"],
        ["fire", "r", "click", True],
        ["set", "g", {"listening": False}],
        ["fire", "r", "click", True],
        ["fire", "c", "click", True],
        ["set", "g", {"listening": True}],
        ["off", "r", ".ns"],
        ["fire", "r", "tap", True],
        ["fire", "r", "click", True],
        ["on", "r", "click.konva", "konva-named", False],
        ["off", "r", "click"],
        ["fire", "r", "click", False],
        ["off", "r", "click.konva"],
        ["fire", "r", "click", False],
        ["on", "r", "click.a click.b dblclick.a", "multi", False],
        ["off", "r", "click.a"],
        ["fire", "r", "click", False],
        ["fire", "r", "dblclick", False],
        ["off", "r", None],
        ["fire", "r", "click", False],
        ["fire", "r", "dblclick", False],
        ["on", "r", "xChange widthChange opacityChange visibleChange nameChange", "change", False],
        ["set", "r", {"x": 5, "width": 10, "opacity": 0.5, "visible": False, "name": "a b"}],
        ["set", "r", {"x": 5, "width": 11, "opacity": 0.5, "visible": True, "name": "a b"}],
        ["call", "r", "move", {"x": 3, "y": 4}],
        ["call", "r", "rotate", 30],
        ["on", "c", "radiusChange widthChange heightChange", "circle", False],
        ["set", "c", {"width": 20}],
        ["set", "c", {"height": 30, "radius": 15}],
        ["get", "c", "size"],
        ["get", "c", "attrs"],
        ["get", "r", "attrs"],
    ]
    return ops


def pairs(groups):
    """Writes the attributes of `new` and `set` as ordered pairs."""
    for group in groups:
        for op in group["ops"]:
            if op[0] == "new":
                op[3] = list(op[3].items())
            elif op[0] == "set":
                op[2] = list(op[2].items())


def main():
    rng = random.Random(10_9_1)
    groups = [{"name": "transform", "ops": transform_cases(rng)}]
    for i in range(8):
        groups.append({"name": f"tree-{i}", "ops": random_tree(rng)})
    groups.append({"name": "collapsed", "ops": collapsed_cases()})
    groups.append({"name": "tree-ops", "ops": tree_ops_cases()})
    groups.append({"name": "events", "ops": events_cases()})
    pairs(groups)
    with open(OUT, "w", newline="\n") as f:
        json.dump({"groups": groups}, f, separators=(",", ":"))
        f.write("\n")
    print(f"wrote {len(groups)} groups, {sum(len(g['ops']) for g in groups)} ops")


if __name__ == "__main__":
    main()
