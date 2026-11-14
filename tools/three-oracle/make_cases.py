"""Writes samples/three/tests/fixtures/cases.json: seeded scripts that
tools/three-oracle/oracle.mjs runs on three.js 0.187.0 and
samples/three/tests/oracle.rs runs on the port.

Each group is a list of ops. Math ops are stateless; camera ops act on one
PerspectiveCamera; orbit ops act on one OrbitControls and its camera.

    python -I tools/three-oracle/make_cases.py
"""

import json
import math
import os
import random

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, 'samples', 'three', 'tests', 'fixtures', 'cases.json')
ORDERS = ['XYZ', 'YXZ', 'ZXY', 'ZYX', 'YZX', 'XZY']

rng = random.Random(187)


def num(lo, hi, digits=None):
    v = rng.uniform(lo, hi)
    return round(v, digits) if digits is not None else v


def vec(lo=-10, hi=10):
    return [num(lo, hi) for _ in range(3)]


def unit():
    while True:
        v = vec(-1, 1)
        n = math.sqrt(sum(c * c for c in v))
        if n > 0.1:
            return [c / n for c in v]


def quat():
    while True:
        q = [num(-1, 1) for _ in range(4)]
        n = math.sqrt(sum(c * c for c in q))
        if n > 0.1:
            return [c / n for c in q]


def angle():
    return rng.choice([num(-math.pi, math.pi), num(-10, 10), rng.choice([0, math.pi / 2, -math.pi / 2, math.pi])])


def math_ops():
    ops = []
    for _ in range(120):
        ops.append(['eulerToQuat', angle(), angle(), angle(), rng.choice(ORDERS)])
    for _ in range(120):
        ops.append(['quatToEuler', *quat(), rng.choice(ORDERS)])
    # Gimbal lock: a quarter turn about the middle axis of each order.
    for order in ORDERS:
        for sign in (1, -1):
            axis = {'X': 0, 'Y': 1, 'Z': 2}[order[1]]
            e = [num(-1, 1), num(-1, 1), num(-1, 1)]
            e[axis] = sign * math.pi / 2
            ops.append(['eulerRoundTrip', *e, order])
    for _ in range(60):
        ops.append(['lookAtMatrix', vec(), vec(), unit()])
    ops.append(['lookAtMatrix', [1, 2, 3], [1, 2, 3], [0, 1, 0]])
    ops.append(['lookAtMatrix', [0, 5, 0], [0, 0, 0], [0, 1, 0]])
    ops.append(['lookAtMatrix', [0, 0, 5], [0, 0, 0], [0, 0, 1]])
    ops.append(['lookAtMatrix', [0, 0, 5], [0, 0, 0], [0, 0, -1]])
    for _ in range(60):
        ops.append(['quatFromRotation', quat()])
    for _ in range(40):
        ops.append(['unitVectors', unit(), unit()])
    for v in ([0, 1, 0], [1, 0, 0], [0, 0, 1]):
        ops.append(['unitVectors', v, [-c for c in v]])
    for _ in range(40):
        s = [rng.choice([1, -1]) * num(0.1, 4) for _ in range(3)]
        if rng.random() < 0.3:
            s = [1, 1, 1]
        ops.append(['composeDecompose', vec(), quat(), s])
    ops.append(['composeDecompose', [1, 2, 3], [0, 0, 0, 1], [0, 1, 1]])
    for _ in range(30):
        m = [num(-3, 3) for _ in range(16)]
        ops.append(['invert', m])
    ops.append(['invert', [1, 2, 3, 4] * 4])
    for _ in range(30):
        ops.append(['multiply', [num(-3, 3) for _ in range(16)], [num(-3, 3) for _ in range(16)]])
    for _ in range(60):
        ops.append(['spherical', vec()])
    ops.append(['spherical', [0, 0, 0]])
    ops.append(['spherical', [0, 3, 0]])
    ops.append(['spherical', [0, -3, 0]])
    for _ in range(40):
        ops.append(['fromSpherical', num(0, 20), num(-1, 4), angle()])
    for _ in range(40):
        ops.append(['applyQuat', vec(), quat()])
    for _ in range(30):
        lo = num(0, 5)
        ops.append(['clampLength', vec(), lo, rng.choice([lo + num(0, 5), float('inf')])])
    ops.append(['clampLength', [0, 0, 0], 1, 2])
    for _ in range(30):
        ops.append(['transformDirection', vec(), [num(-3, 3) for _ in range(16)]])
    return ops


def camera_ops():
    ops = []
    for _ in range(12):
        ops.append(['camera', num(10, 120), num(0.3, 3), num(0.01, 1), num(10, 5000)])
        for _ in range(10):
            kind = rng.choice(['set', 'set', 'view', 'focal', 'effective', 'pos', 'rot', 'lookAt', 'up'])
            if kind == 'set':
                prop = rng.choice(['fov', 'aspect', 'near', 'far', 'zoom', 'filmGauge', 'filmOffset', 'coordinateSystem'])
                value = {
                    'fov': num(5, 170),
                    'aspect': num(0.2, 4),
                    'near': num(0.001, 2),
                    'far': num(5, 10000),
                    'zoom': num(0.1, 8),
                    'filmGauge': num(10, 70),
                    'filmOffset': num(-5, 5),
                    'coordinateSystem': rng.choice(['webgl', 'webgpu']),
                }[prop]
                ops.append(['cset', prop, value])
                ops.append(['cupdate'])
            elif kind == 'view':
                fw, fh = num(100, 2000, 0), num(100, 2000, 0)
                ops.append(['cview', fw, fh, num(0, fw / 2, 0), num(0, fh / 2, 0), num(10, fw / 2, 0), num(10, fh / 2, 0)])
                if rng.random() < 0.3:
                    ops.append(['cclearView'])
            elif kind == 'focal':
                ops.append(['cfocal', num(10, 200)])
            elif kind == 'effective':
                ops.append(['ceffective'])
            elif kind == 'pos':
                ops.append(['cpos', *vec()])
            elif kind == 'rot':
                ops.append(['crot', angle(), angle(), angle(), rng.choice(ORDERS)])
            elif kind == 'up':
                ops.append(['cup', *unit()])
            else:
                ops.append(['clookAt', *vec()])
    return ops


OPTION_VALUES = {
    'enableDamping': lambda: rng.choice([True, False]),
    'dampingFactor': lambda: num(0.01, 0.5),
    'minDistance': lambda: num(0, 5),
    'maxDistance': lambda: rng.choice([num(5, 40), float('inf')]),
    'minPolarAngle': lambda: num(0, 1.2),
    'maxPolarAngle': lambda: num(1.6, math.pi),
    'minAzimuthAngle': lambda: rng.choice([num(-7, 0), float('-inf')]),
    'maxAzimuthAngle': lambda: rng.choice([num(0, 7), num(-7, -3), float('inf')]),
    'minTargetRadius': lambda: num(0, 2),
    'maxTargetRadius': lambda: rng.choice([num(2, 10), float('inf')]),
    'zoomSpeed': lambda: num(0.2, 3),
    'rotateSpeed': lambda: num(0.2, 3),
    'keyRotateSpeed': lambda: num(0.2, 3),
    'panSpeed': lambda: num(0.2, 3),
    'keyPanSpeed': lambda: num(1, 20),
    'screenSpacePanning': lambda: rng.choice([True, False]),
    'autoRotate': lambda: rng.choice([True, False]),
    'autoRotateSpeed': lambda: num(-5, 5),
    'enableZoom': lambda: rng.random() < 0.8,
    'enableRotate': lambda: rng.random() < 0.8,
    'enablePan': lambda: rng.random() < 0.8,
    'enabled': lambda: rng.random() < 0.9,
    'target': lambda: vec(-3, 3),
    'cursor': lambda: vec(-2, 2),
    'mouseButtons': lambda: [rng.choice([0, 1, 2, None]) for _ in range(3)],
}


def mods():
    m = {}
    for k in ('ctrl', 'meta', 'shift'):
        if rng.random() < 0.15:
            m[k] = True
    return m


def gesture(width, height):
    kind = rng.choice(['drag', 'drag', 'drag', 'wheel', 'key', 'call', 'update'])
    ops = []
    if kind == 'drag':
        button = rng.choice([0, 0, 1, 2])
        x, y = num(0, width, 0), num(0, height, 0)
        m = mods()
        ops.append(['down', button, x, y, m])
        for _ in range(rng.randint(1, 5)):
            x += num(-80, 80, 0)
            y += num(-80, 80, 0)
            ops.append(['move', x, y, m])
        ops.append(['up', x, y])
    elif kind == 'wheel':
        for _ in range(rng.randint(1, 3)):
            ops.append(['wheel', num(0, width, 0), num(0, height, 0), rng.choice([num(-300, 300), 100, -100, 3, -3, 0]), rng.choice([0, 0, 1, 2]), rng.random() < 0.2])
    elif kind == 'key':
        if rng.random() < 0.2:
            ops.append(['controlKey', True])
        ops.append(['key', rng.choice(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'KeyA']), mods()])
        if rng.random() < 0.5:
            ops.append(['controlKey', False])
    elif kind == 'call':
        call = rng.choice(['rotateLeft', 'rotateUp', 'pan', 'dollyIn', 'dollyOut', 'save', 'reset'])
        if call in ('rotateLeft', 'rotateUp'):
            ops.append([call, num(-2, 2)])
        elif call == 'pan':
            ops.append([call, num(-100, 100), num(-100, 100)])
        elif call in ('dollyIn', 'dollyOut'):
            ops.append([call, num(0.5, 2)])
        else:
            ops.append([call])
    else:
        for _ in range(rng.randint(1, 6)):
            ops.append(['update', rng.choice([None, num(0, 0.1)])])
    return ops


def orbit_ops():
    groups = []
    for i in range(40):
        width, height = num(200, 1600, 0), num(150, 1000, 0)
        up = [0, 1, 0] if i % 4 else rng.choice([[0, 0, 1], unit()])
        config = {
            'fov': num(20, 90),
            'aspect': width / height,
            'near': 0.1,
            'far': 1000,
            'position': vec(-20, 20),
            'up': up,
            'width': width,
            'height': height,
        }
        ops = [['orbit', config]]
        for _ in range(rng.randint(2, 6)):
            prop = rng.choice(list(OPTION_VALUES))
            ops.append(['set', prop, OPTION_VALUES[prop]()])
        for _ in range(rng.randint(6, 14)):
            if rng.random() < 0.15:
                prop = rng.choice(list(OPTION_VALUES))
                ops.append(['set', prop, OPTION_VALUES[prop]()])
            ops.extend(gesture(width, height))
        groups.append({'name': f'orbit-{i}', 'ops': ops})
    return groups


def main():
    groups = [{'name': 'math', 'ops': math_ops()}, {'name': 'camera', 'ops': camera_ops()}]
    groups.extend(orbit_ops())

    def enc(v):
        if isinstance(v, float):
            if math.isnan(v):
                return 'NaN'
            if math.isinf(v):
                return 'Infinity' if v > 0 else '-Infinity'
        if isinstance(v, list):
            return [enc(x) for x in v]
        if isinstance(v, dict):
            return {k: enc(x) for k, x in v.items()}
        return v

    data = {
        'comment': 'Generated by tools/three-oracle/make_cases.py; do not edit.',
        'groups': [{'name': g['name'], 'ops': enc(g['ops'])} for g in groups],
    }
    with open(OUT, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(data, f, separators=(',', ':'))
        f.write('\n')
    print(sum(len(g['ops']) for g in groups), 'ops in', len(groups), 'groups')


main()
