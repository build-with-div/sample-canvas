# three

A port of [three.js](https://threejs.org) 0.187.0's camera math and
`OrbitControls` to Rust (milestone P2-6a). It is the part of a three.js
viewer that needs no platform feature, and it is what the three.js shell
(P2-6b) and the WebGPU scene (P2-6c, waiting for W17) will drive.

**Math** (`src/math.rs`): `Vector2`, `Vector3`, `Quaternion`, `Euler` (all
six orders), `Matrix4` (`compose`, `decompose`, `lookAt`,
`makeRotationFromQuaternion`, `multiplyMatrices`, `invert`,
`makePerspective` in the WebGL and WebGPU coordinate systems) and
`Spherical`, operation for operation as three.js computes them.

**Camera** (`src/camera.rs`): `PerspectiveCamera` with its projection
matrix and inverse, `zoom`, `filmGauge`, `filmOffset`, `setFocalLength`,
`getEffectiveFOV`, view offsets, `lookAt`, and the object matrices
(`updateMatrix`, `updateMatrixWorld`, `matrixWorldInverse`).

**OrbitControls** (`src/orbit.rs`): rotate, dolly and pan by mouse, wheel
and keys, with damping, auto-rotation, distance, polar, azimuth and target
limits, `mouseButtons`, `saveState` and `reset`, and the `change`, `start`
and `end` events. Input arrives as plain structs carrying the fields
OrbitControls reads; the element is its client size. Not ported: touch,
`zoomToCursor`, orthographic cameras and cursor styles.

**`Math.sin`, `Math.cos` and `Math.pow`** (`src/jsmath.rs`): three.js runs
in Chromium, whose sine and cosine are correctly rounded except for some
near-ties and whose `pow` is correctly rounded except for exact ties.
`libm` is not always correctly rounded, and a 1-ulp difference spreads
through the orbit state. The port evaluates these three in double-double
arithmetic and rounds once. The rest (`tan`, `asin`, `acos`, `atan`,
`atan2`) comes from `libm`, which matched Chromium on every call measured.

## Fidelity

`tests/oracle.rs` runs the scripts in `tests/fixtures/cases.json` on the
port and compares every result with three.js 0.187.0 running the same
scripts in Chrome for Testing 147.0.7727.50
(`tests/fixtures/three-0.187.0.json`): numbers within 2 ulps, everything
else exactly. Differences beyond that are listed with their cause in
`tests/fixtures/three-differences.txt`. See
[docs/phase2-plan.md](../../docs/phase2-plan.md) (P2-6a) for the
measurements.

```
cargo test -p three --test oracle -- --nocapture
```

To re-record, with `CHROME` the `chrome` binary of Chrome for Testing
147.0.7727.50:

```
python -I tools/three-oracle/make_cases.py
npm ci --prefix tools/three-oracle
node tools/three-oracle/oracle.mjs CHROME
```

three.js is MIT licensed; see [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
