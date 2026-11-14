//! The port compared with three.js 0.187.0 itself.
//!
//! `fixtures/cases.json` (from `tools/three-oracle/make_cases.py`) holds
//! scripts of math, camera and OrbitControls operations;
//! `fixtures/three-0.187.0.json` is what three.js returned for each in
//! pinned Chromium (`tools/three-oracle/oracle.mjs`). This test runs the same scripts on the
//! port and compares every result: numbers within `MAX_ULPS`, everything
//! else exactly. A difference beyond that must be listed, with its cause, in
//! `fixtures/three-differences.txt`; a listed one that no longer occurs
//! fails too.

use serde_json::{json, Map, Value as Json};
use std::collections::BTreeSet;
use three::{
    ControlsEvent, CoordinateSystem, Element, Euler, EulerOrder, Matrix4, Modifiers, MouseAction,
    OrbitControls, PerspectiveCamera, PointerEvent, Quaternion, Spherical, Vector3, WheelEvent,
};

const MAX_ULPS: u64 = 2;

/// Numbers as the oracle writes them: NaN, the infinities and -0 as
/// strings, because JSON has no form for them.
fn num(v: f64) -> Json {
    if v.is_nan() {
        json!("NaN")
    } else if v == f64::INFINITY {
        json!("Infinity")
    } else if v == f64::NEG_INFINITY {
        json!("-Infinity")
    } else if v == 0.0 && v.is_sign_negative() {
        json!("-0")
    } else {
        json!(v)
    }
}

fn decode(v: &Json) -> Option<f64> {
    match v {
        Json::Number(n) => n.as_f64(),
        Json::String(s) => match s.as_str() {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

fn f(v: &Json) -> f64 {
    decode(v).unwrap_or_else(|| panic!("a number, not {v}"))
}

fn nums(values: &[f64]) -> Json {
    Json::Array(values.iter().map(|&v| num(v)).collect())
}

fn v3(v: Vector3) -> Json {
    nums(&[v.x, v.y, v.z])
}

fn q4(q: Quaternion) -> Json {
    nums(&[q.x, q.y, q.z, q.w])
}

fn e3(e: Euler) -> Json {
    nums(&[e.x, e.y, e.z])
}

fn m16(m: &Matrix4) -> Json {
    nums(&m.elements)
}

fn vec_of(v: &Json) -> Vector3 {
    Vector3::new(f(&v[0]), f(&v[1]), f(&v[2]))
}

fn quat_of(v: &Json) -> Quaternion {
    Quaternion {
        x: f(&v[0]),
        y: f(&v[1]),
        z: f(&v[2]),
        w: f(&v[3]),
    }
}

/// `new Matrix4().fromArray(a)`: column-major.
fn matrix_of(v: &Json) -> Matrix4 {
    let mut m = Matrix4::default();
    for (i, e) in m.elements.iter_mut().enumerate() {
        *e = f(&v[i]);
    }
    m
}

fn order(v: &Json) -> EulerOrder {
    EulerOrder::parse(v.as_str().unwrap()).unwrap()
}

fn math_op(op: &[Json]) -> Json {
    let a = &op[1..];
    match op[0].as_str().unwrap() {
        "eulerToQuat" => {
            let mut q = Quaternion::default();
            q.set_from_euler(&Euler {
                x: f(&a[0]),
                y: f(&a[1]),
                z: f(&a[2]),
                order: order(&a[3]),
            });
            q4(q)
        }
        "quatToEuler" => {
            let mut e = Euler::default();
            e.set_from_quaternion(&quat_of(&Json::Array(a[..4].to_vec())), order(&a[4]));
            e3(e)
        }
        "eulerRoundTrip" => {
            let o = order(&a[3]);
            let mut q = Quaternion::default();
            q.set_from_euler(&Euler {
                x: f(&a[0]),
                y: f(&a[1]),
                z: f(&a[2]),
                order: o,
            });
            let mut e = Euler::default();
            e.set_from_quaternion(&q, o);
            json!([q4(q), e3(e)])
        }
        "lookAtMatrix" => {
            let mut m = Matrix4::default();
            m.look_at(vec_of(&a[0]), vec_of(&a[1]), vec_of(&a[2]));
            m16(&m)
        }
        "quatFromRotation" => {
            let mut m = Matrix4::default();
            m.make_rotation_from_quaternion(&quat_of(&a[0]));
            let mut q = Quaternion::default();
            q.set_from_rotation_matrix(&m);
            json!([m16(&m), q4(q)])
        }
        "unitVectors" => {
            let mut q = Quaternion::default();
            q.set_from_unit_vectors(vec_of(&a[0]), vec_of(&a[1]));
            q4(q)
        }
        "composeDecompose" => {
            let mut m = Matrix4::default();
            m.compose(vec_of(&a[0]), &quat_of(&a[1]), vec_of(&a[2]));
            let (p, q, s) = m.decompose();
            json!([m16(&m), v3(p), q4(q), v3(s)])
        }
        "invert" => {
            let mut m = matrix_of(&a[0]);
            m.invert();
            m16(&m)
        }
        "multiply" => {
            let mut m = Matrix4::default();
            m.multiply_matrices(&matrix_of(&a[0]), &matrix_of(&a[1]));
            m16(&m)
        }
        "spherical" => {
            let mut s = Spherical::default();
            s.set_from_vector3(vec_of(&a[0]));
            let raw = nums(&[s.radius, s.phi, s.theta]);
            s.make_safe();
            json!([raw, num(s.phi)])
        }
        "fromSpherical" => {
            let mut v = Vector3::default();
            v.set_from_spherical_coords(f(&a[0]), f(&a[1]), f(&a[2]));
            v3(v)
        }
        "applyQuat" => {
            let mut v = vec_of(&a[0]);
            v.apply_quaternion(&quat_of(&a[1]));
            v3(v)
        }
        "clampLength" => {
            let mut v = vec_of(&a[0]);
            v.clamp_length(f(&a[1]), f(&a[2]));
            v3(v)
        }
        "transformDirection" => {
            let mut v = vec_of(&a[0]);
            v.transform_direction(&matrix_of(&a[1]));
            v3(v)
        }
        other => panic!("unknown math op {other}"),
    }
}

fn projection(c: &PerspectiveCamera) -> Map<String, Json> {
    let mut m = Map::new();
    m.insert("projection".into(), m16(&c.projection_matrix));
    m.insert("inverse".into(), m16(&c.projection_matrix_inverse));
    m
}

fn orientation(c: &PerspectiveCamera) -> Map<String, Json> {
    let mut m = Map::new();
    m.insert("quaternion".into(), q4(c.quaternion()));
    m.insert("rotation".into(), e3(c.rotation()));
    m
}

fn camera_op(camera: &mut Option<PerspectiveCamera>, op: &[Json]) -> Json {
    let a = &op[1..];
    let kind = op[0].as_str().unwrap();
    if kind == "camera" {
        let c = PerspectiveCamera::new(f(&a[0]), f(&a[1]), f(&a[2]), f(&a[3]));
        let out = projection(&c);
        *camera = Some(c);
        return Json::Object(out);
    }
    let c = camera.as_mut().expect("a camera");
    let mut out = Map::new();
    match kind {
        "cset" => {
            let prop = a[0].as_str().unwrap();
            if prop == "coordinateSystem" {
                c.coordinate_system = if a[1] == "webgpu" {
                    CoordinateSystem::WebGpu
                } else {
                    CoordinateSystem::WebGl
                };
            } else {
                let v = f(&a[1]);
                match prop {
                    "fov" => c.fov = v,
                    "aspect" => c.aspect = v,
                    "near" => c.near = v,
                    "far" => c.far = v,
                    "zoom" => c.zoom = v,
                    "filmGauge" => c.film_gauge = v,
                    "filmOffset" => c.film_offset = v,
                    other => panic!("unknown camera property {other}"),
                }
            }
        }
        "cupdate" => {
            c.update_projection_matrix();
            out = projection(c);
        }
        "cview" => {
            let v: Vec<f64> = a.iter().map(f).collect();
            c.set_view_offset(v[0], v[1], v[2], v[3], v[4], v[5]);
            out.insert("aspect".into(), num(c.aspect));
            out.extend(projection(c));
        }
        "cclearView" => {
            c.clear_view_offset();
            out = projection(c);
        }
        "cfocal" => {
            c.set_focal_length(f(&a[0]));
            out.insert("fov".into(), num(c.fov));
            out.extend(projection(c));
        }
        "ceffective" => {
            out.insert("effective".into(), num(c.effective_fov()));
            out.insert("filmWidth".into(), num(c.film_width()));
            out.insert("filmHeight".into(), num(c.film_height()));
        }
        "cpos" => c.position = Vector3::new(f(&a[0]), f(&a[1]), f(&a[2])),
        "cup" => c.up = Vector3::new(f(&a[0]), f(&a[1]), f(&a[2])),
        "crot" => {
            c.set_rotation(f(&a[0]), f(&a[1]), f(&a[2]), order(&a[3]));
            out = orientation(c);
        }
        "clookAt" => {
            c.look_at(Vector3::new(f(&a[0]), f(&a[1]), f(&a[2])));
            out = orientation(c);
            out.insert("world".into(), m16(&c.matrix_world));
            out.insert("worldInverse".into(), m16(&c.matrix_world_inverse));
        }
        other => panic!("unknown camera op {other}"),
    }
    Json::Object(out)
}

/// The page's `orbitState()`: the controls' and camera's state, and the
/// events dispatched since the last one.
fn orbit_state(controls: &mut OrbitControls, returned: Option<bool>) -> Json {
    let c = &controls.camera;
    let mut out = Map::new();
    out.insert("state".into(), json!(controls.state.code()));
    out.insert("position".into(), v3(c.position));
    out.insert("quaternion".into(), q4(c.quaternion()));
    out.insert("rotation".into(), e3(c.rotation()));
    out.insert("target".into(), v3(controls.target));
    out.insert("matrix".into(), m16(&c.matrix));
    out.insert("worldInverse".into(), m16(&c.matrix_world_inverse));
    out.insert("polar".into(), num(controls.polar_angle()));
    out.insert("azimuth".into(), num(controls.azimuthal_angle()));
    out.insert("distance".into(), num(controls.distance()));
    out.insert("zoom".into(), num(c.zoom));
    let events: Vec<Json> = controls
        .events
        .drain(..)
        .map(|e: ControlsEvent| json!(e.as_str()))
        .collect();
    out.insert("events".into(), Json::Array(events));
    if let Some(r) = returned {
        out.insert("returned".into(), json!(r));
    }
    Json::Object(out)
}

fn modifiers(v: &Json) -> Modifiers {
    let flag = |k: &str| v.get(k).and_then(Json::as_bool).unwrap_or(false);
    Modifiers {
        ctrl: flag("ctrl"),
        meta: flag("meta"),
        shift: flag("shift"),
    }
}

fn pointer(button: i32, x: &Json, y: &Json, mods: Option<&Json>) -> PointerEvent {
    PointerEvent {
        pointer_id: 1,
        button,
        client_x: f(x),
        client_y: f(y),
        modifiers: mods.map(modifiers).unwrap_or_default(),
    }
}

fn mouse_action(v: &Json) -> Option<MouseAction> {
    match v.as_i64() {
        Some(0) => Some(MouseAction::Rotate),
        Some(1) => Some(MouseAction::Dolly),
        Some(2) => Some(MouseAction::Pan),
        _ => None,
    }
}

fn set_option(controls: &mut OrbitControls, prop: &str, value: &Json) {
    let flag = || value.as_bool().unwrap();
    match prop {
        "target" => controls.target = vec_of(value),
        "cursor" => controls.cursor = vec_of(value),
        "mouseButtons" => {
            for (slot, v) in controls
                .mouse_buttons
                .iter_mut()
                .zip(value.as_array().unwrap())
            {
                *slot = mouse_action(v);
            }
        }
        "enableDamping" => controls.enable_damping = flag(),
        "screenSpacePanning" => controls.screen_space_panning = flag(),
        "autoRotate" => controls.auto_rotate = flag(),
        "enableZoom" => controls.enable_zoom = flag(),
        "enableRotate" => controls.enable_rotate = flag(),
        "enablePan" => controls.enable_pan = flag(),
        "enabled" => controls.enabled = flag(),
        _ => {
            let v = f(value);
            let field = match prop {
                "dampingFactor" => &mut controls.damping_factor,
                "minDistance" => &mut controls.min_distance,
                "maxDistance" => &mut controls.max_distance,
                "minPolarAngle" => &mut controls.min_polar_angle,
                "maxPolarAngle" => &mut controls.max_polar_angle,
                "minAzimuthAngle" => &mut controls.min_azimuth_angle,
                "maxAzimuthAngle" => &mut controls.max_azimuth_angle,
                "minTargetRadius" => &mut controls.min_target_radius,
                "maxTargetRadius" => &mut controls.max_target_radius,
                "zoomSpeed" => &mut controls.zoom_speed,
                "rotateSpeed" => &mut controls.rotate_speed,
                "keyRotateSpeed" => &mut controls.key_rotate_speed,
                "panSpeed" => &mut controls.pan_speed,
                "keyPanSpeed" => &mut controls.key_pan_speed,
                "autoRotateSpeed" => &mut controls.auto_rotate_speed,
                other => panic!("unknown option {other}"),
            };
            *field = v;
        }
    }
}

fn orbit_op(controls: &mut Option<OrbitControls>, op: &[Json]) -> Json {
    let a = &op[1..];
    let kind = op[0].as_str().unwrap();
    if kind == "orbit" {
        let config = &a[0];
        let mut camera = PerspectiveCamera::new(
            f(&config["fov"]),
            f(&config["aspect"]),
            f(&config["near"]),
            f(&config["far"]),
        );
        camera.position = vec_of(&config["position"]);
        camera.up = vec_of(&config["up"]);
        let element = Element {
            client_width: f(&config["width"]),
            client_height: f(&config["height"]),
        };
        let mut c = OrbitControls::new(camera, element);
        let out = orbit_state(&mut c, None);
        *controls = Some(c);
        return out;
    }
    let c = controls.as_mut().expect("controls");
    match kind {
        "set" => {
            set_option(c, a[0].as_str().unwrap(), &a[1]);
            json!({})
        }
        "down" => {
            let button = a[0].as_i64().unwrap() as i32;
            c.pointer_down(&pointer(button, &a[1], &a[2], a.get(3)));
            orbit_state(c, None)
        }
        "move" => {
            c.pointer_move(&pointer(0, &a[0], &a[1], a.get(2)));
            orbit_state(c, None)
        }
        "up" => {
            c.pointer_up(&pointer(0, &a[0], &a[1], None));
            orbit_state(c, None)
        }
        "wheel" => {
            c.wheel(&WheelEvent {
                client_x: f(&a[0]),
                client_y: f(&a[1]),
                delta_y: f(&a[2]),
                delta_mode: a[3].as_u64().unwrap() as u32,
                ctrl: a[4].as_bool().unwrap(),
            });
            orbit_state(c, None)
        }
        "key" => {
            c.key_down(a[0].as_str().unwrap(), modifiers(&a[1]));
            orbit_state(c, None)
        }
        "controlKey" => {
            c.control_key(a[0].as_bool().unwrap());
            json!({})
        }
        "update" => {
            let dt = if a[0].is_null() { None } else { Some(f(&a[0])) };
            let returned = c.update(dt);
            orbit_state(c, Some(returned))
        }
        "rotateLeft" => {
            c.rotate_left(f(&a[0]));
            orbit_state(c, None)
        }
        "rotateUp" => {
            c.rotate_up(f(&a[0]));
            orbit_state(c, None)
        }
        "dollyIn" => {
            c.dolly_in(f(&a[0]));
            orbit_state(c, None)
        }
        "dollyOut" => {
            c.dolly_out(f(&a[0]));
            orbit_state(c, None)
        }
        "pan" => {
            c.pan(f(&a[0]), f(&a[1]));
            orbit_state(c, None)
        }
        "save" => {
            c.save_state();
            json!({})
        }
        "reset" => {
            c.reset();
            orbit_state(c, None)
        }
        other => panic!("unknown orbit op {other}"),
    }
}

/// Distance in units in the last place, with -0 and +0 one apart.
fn ulps(a: f64, b: f64) -> u64 {
    fn key(v: f64) -> i64 {
        let bits = v.to_bits() as i64;
        if bits < 0 {
            i64::MIN - bits
        } else {
            bits
        }
    }
    key(a).abs_diff(key(b))
}

#[derive(Default)]
struct Tally {
    numbers: usize,
    identical: usize,
    within: usize,
    max_ulps: u64,
    max_abs_diff: f64,
    others: usize,
    failures: Vec<String>,
    /// The allowed differences, by `<group> op <index><path>`.
    known: BTreeSet<String>,
    seen_known: Vec<String>,
    op: String,
}

impl Tally {
    fn fail(&mut self, ctx: &str, what: String) {
        if self.known.contains(ctx) {
            self.seen_known.push(format!("{ctx}: {what}"));
            return;
        }
        if self.failures.len() < 40 {
            self.failures
                .push(format!("{ctx}: {what} (op {})", self.op));
        } else if self.failures.len() == 40 {
            self.failures.push("…".into());
        }
    }

    fn compare(&mut self, ctx: &str, got: &Json, want: &Json) {
        if let (Some(g), Some(w)) = (decode(got), decode(want)) {
            self.numbers += 1;
            if g.to_bits() == w.to_bits() || (g.is_nan() && w.is_nan()) {
                self.identical += 1;
                return;
            }
            let d = if g.is_finite() && w.is_finite() {
                ulps(g, w)
            } else {
                u64::MAX
            };
            if d > MAX_ULPS {
                self.fail(ctx, format!("port {g:?}, three.js {w:?} ({d} ulps)"));
            } else {
                self.within += 1;
                self.max_ulps = self.max_ulps.max(d);
                self.max_abs_diff = self.max_abs_diff.max((g - w).abs());
            }
            return;
        }
        match (got, want) {
            (Json::Array(g), Json::Array(w)) => {
                if g.len() != w.len() {
                    self.fail(ctx, format!("port {got}, three.js {want}"));
                    return;
                }
                for (i, (g, w)) in g.iter().zip(w).enumerate() {
                    self.compare(&format!("{ctx}[{i}]"), g, w);
                }
            }
            (Json::Object(g), Json::Object(w)) => {
                let mut gk: Vec<_> = g.keys().collect();
                let mut wk: Vec<_> = w.keys().collect();
                gk.sort();
                wk.sort();
                if gk != wk {
                    self.fail(ctx, format!("port {got}, three.js {want}"));
                    return;
                }
                for (k, w) in w {
                    self.compare(&format!("{ctx}.{k}"), &g[k], w);
                }
            }
            _ => {
                self.others += 1;
                if got != want {
                    self.fail(ctx, format!("port {got}, three.js {want}"));
                }
            }
        }
    }
}

#[test]
fn camera_and_orbit_controls_match_three_0_187_0() {
    let cases: Json = serde_json::from_str(include_str!("fixtures/cases.json")).unwrap();
    let oracle: Json = serde_json::from_str(include_str!("fixtures/three-0.187.0.json")).unwrap();
    assert_eq!(oracle["three"], "0.187.0");
    let groups = cases["groups"].as_array().unwrap();
    let recorded = oracle["groups"].as_array().unwrap();
    assert_eq!(groups.len(), recorded.len());
    let mut tally = Tally {
        known: include_str!("fixtures/three-differences.txt")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect(),
        ..Tally::default()
    };
    let (mut ops, mut events) = (0, 0);
    for (group, want) in groups.iter().zip(recorded) {
        let name = group["name"].as_str().unwrap();
        assert_eq!(want["name"], name);
        let want = want["results"].as_array().unwrap();
        let script = group["ops"].as_array().unwrap();
        assert_eq!(script.len(), want.len(), "{name}");
        let mut camera = None;
        let mut controls = None;
        for (i, (op, w)) in script.iter().zip(want).enumerate() {
            let op = op.as_array().unwrap();
            let got = match name {
                "math" => math_op(op),
                "camera" => camera_op(&mut camera, op),
                _ => orbit_op(&mut controls, op),
            };
            ops += 1;
            events += w["events"].as_array().map_or(0, |e| e.len());
            tally.op = Json::Array(op.clone()).to_string();
            tally.compare(&format!("{name} op {i}"), &got, w);
        }
    }
    println!(
        "THREE oracle: {ops} ops, {events} controls events; {} of {} numbers identical, {} within {} ulps \
         (max abs diff {:e}), {} listed; {} other values",
        tally.identical,
        tally.numbers,
        tally.within,
        tally.max_ulps,
        tally.max_abs_diff,
        tally.seen_known.len(),
        tally.others
    );
    for known in &tally.seen_known {
        println!("  listed: {known}");
    }
    for failure in &tally.failures {
        println!("  {failure}");
    }
    let seen: BTreeSet<&str> = tally
        .seen_known
        .iter()
        .map(|k| k.split(": ").next().unwrap())
        .collect();
    let gone: Vec<&String> = tally
        .known
        .iter()
        .filter(|k| !seen.contains(k.as_str()))
        .collect();
    assert!(
        gone.is_empty(),
        "listed differences no longer occur: {gone:?}"
    );
    assert!(
        tally.failures.is_empty(),
        "the port differs from three.js 0.187.0"
    );
}
