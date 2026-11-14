//! Runs the scripts in `fixtures/cases.json` on the port and compares every
//! result, every event and every 2D context call with what Konva 10.9.1 did
//! for the same script (`fixtures/konva-10.9.1.json`, recorded by
//! `tools/konva-oracle/oracle.mjs`).
//!
//! Structure, strings, booleans, handles, event order and the order of
//! context calls on each canvas must be identical.
//! A number must be identical or within `MAX_ULPS` units in the last place;
//! the test prints how many were identical. The differences listed in
//! `fixtures/konva-differences.txt`, each under the cause it comes from, are
//! allowed; the test fails on any other, and on a listed one that no longer
//! occurs.

use konva::{
    clear_color_keys, color_to_rgba, js, set_math_random, Arg, CanvasCall, CanvasKind, ClassName,
    ClientRectConfig, Decomposed, IRect, KonvaEvent, Node, Transform, Value, Vector2d,
};
use serde_json::{json, Value as Json};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

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

fn value(v: &Value) -> Json {
    match v {
        Value::Number(n) => num(*n),
        Value::Bool(b) => json!(b),
        Value::String(s) => json!(s),
        Value::Array(a) => Json::Array(a.iter().map(|&v| num(v)).collect()),
    }
}

fn from_json(v: &Json) -> Value {
    match v {
        Json::Number(n) => Value::Number(n.as_f64().unwrap()),
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(a) => Value::Array(a.iter().map(|v| v.as_f64().unwrap()).collect()),
        other => panic!("no attribute value for {other}"),
    }
}

fn arg(a: &Arg) -> Json {
    match a {
        Arg::Number(n) => num(*n),
        Arg::Str(s) => json!(s),
        Arg::Bool(b) => json!(b),
        Arg::Numbers(v) => Json::Array(v.iter().map(|&n| num(n)).collect()),
    }
}

/// A call as the oracle writes it: `[method, ...args]`, or
/// `["=property", value]` for an assignment.
fn call(c: &CanvasCall) -> Json {
    match c {
        CanvasCall::Call { method, args } => {
            let mut out = vec![json!(method)];
            out.extend(args.iter().map(arg));
            Json::Array(out)
        }
        CanvasCall::Set { property, value } => json!([format!("={property}"), arg(value)]),
    }
}

/// `mulberry32`, as the oracle seeds `Math.random` with it.
fn mulberry32(seed: i32) -> impl FnMut() -> f64 {
    let mut seed = seed as u32;
    move || {
        seed = seed.wrapping_add(0x6d2b_79f5);
        let mut t = (seed ^ (seed >> 15)).wrapping_mul(1 | seed);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

fn matrix(t: &Transform) -> Json {
    Json::Array(t.m.iter().map(|&v| num(v)).collect())
}

fn vector(v: Vector2d) -> Json {
    json!({ "x": num(v.x), "y": num(v.y) })
}

fn rect(r: IRect) -> Json {
    json!({ "x": num(r.x), "y": num(r.y), "width": num(r.width), "height": num(r.height) })
}

fn decomposed(d: Decomposed) -> Json {
    json!({
        "x": num(d.x),
        "y": num(d.y),
        "rotation": num(d.rotation),
        "scaleX": num(d.scale_x),
        "scaleY": num(d.scale_y),
        "skewX": num(d.skew_x),
        "skewY": num(d.skew_y),
    })
}

fn f(v: &Json) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

fn run_transform(seq: &Json) -> Json {
    let mut t = Transform::new();
    let mut out = Vec::new();
    for op in seq.as_array().unwrap() {
        let op = op.as_array().unwrap();
        let arg = |i: usize| f(&op[i]);
        out.push(match op[0].as_str().unwrap() {
            "translate" => matrix(t.translate(arg(1), arg(2))),
            "scale" => matrix(t.scale(arg(1), arg(2))),
            "skew" => matrix(t.skew(arg(1), arg(2))),
            "rotate" => matrix(t.rotate(arg(1))),
            "multiply" => {
                let m: Vec<f64> = op[1].as_array().unwrap().iter().map(f).collect();
                matrix(t.multiply(&Transform::from_matrix(m.try_into().unwrap())))
            }
            "invert" => matrix(t.invert()),
            "point" => vector(t.point(Vector2d {
                x: arg(1),
                y: arg(2),
            })),
            "decompose" => decomposed(t.decompose()),
            "rect" => rect(t.transformed_rect(IRect {
                x: arg(1),
                y: arg(2),
                width: arg(3),
                height: arg(4),
            })),
            "isInvertible" => json!(t.is_invertible()),
            other => panic!("unknown transform op {other}"),
        });
    }
    Json::Array(out)
}

fn class(name: &str) -> ClassName {
    match name {
        "Stage" => ClassName::Stage,
        "Layer" => ClassName::Layer,
        "Group" => ClassName::Group,
        "Rect" => ClassName::Rect,
        "Circle" => ClassName::Circle,
        other => panic!("unknown class {other}"),
    }
}

fn attr_pairs(v: &Json) -> Vec<(String, Value)> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|pair| (pair[0].as_str().unwrap().to_string(), from_json(&pair[1])))
        .collect()
}

#[derive(Default)]
struct Handles {
    nodes: HashMap<String, Node>,
    names: HashMap<u64, String>,
    /// Nodes with canvases, in creation order.
    with_canvases: Vec<(String, Node)>,
}

impl Handles {
    fn node(&self, h: &Json) -> Node {
        let h = h.as_str().unwrap();
        self.nodes
            .get(h)
            .unwrap_or_else(|| panic!("no node {h}"))
            .clone()
    }

    fn name(&self, n: Option<&Node>) -> Json {
        match n {
            Some(n) => json!(self.names.get(&n.internal_id()).map_or("?", |s| s.as_str())),
            None => Json::Null,
        }
    }
}

fn canvas_name(kind: CanvasKind, stage: bool) -> &'static str {
    match (kind, stage) {
        (CanvasKind::Hit, true) => "bufferHit",
        (CanvasKind::Scene, _) => "scene",
        (CanvasKind::Hit, false) => "hit",
    }
}

fn run_group(seed: i32, ops: &Json) -> Vec<Json> {
    set_math_random(mulberry32(seed));
    clear_color_keys();
    let handles = Rc::new(RefCell::new(Handles::default()));
    let log: Rc<RefCell<Vec<Json>>> = Rc::default();
    let mut results = Vec::new();
    for op in ops.as_array().unwrap() {
        log.borrow_mut().clear();
        let op = op.as_array().unwrap();
        let kind = op[0].as_str().unwrap();
        let node = || handles.borrow().node(&op[1]);
        let mut r = Json::Null;
        match kind {
            "new" => {
                let attrs = attr_pairs(&op[3]);
                let config: Vec<(&str, Value)> =
                    attrs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                let n = Node::new(class(op[2].as_str().unwrap()), &config);
                let h = op[1].as_str().unwrap().to_string();
                let mut handles = handles.borrow_mut();
                handles.names.insert(n.internal_id(), h.clone());
                if !n.canvases().is_empty() {
                    handles.with_canvases.push((h.clone(), n.clone()));
                }
                handles.nodes.insert(h, n);
            }
            "add" => {
                let parent = node();
                for child in op[2].as_array().unwrap() {
                    let child = handles.borrow().node(child);
                    parent.add(&child);
                }
            }
            "set" => {
                let attrs = attr_pairs(&op[2]);
                let config: Vec<(&str, Value)> =
                    attrs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                node().set_attrs(&config);
            }
            "call" => {
                let n = node();
                let arg = &op[3];
                match op[2].as_str().unwrap() {
                    "moveToTop" => r = json!(n.move_to_top()),
                    "moveUp" => r = json!(n.move_up()),
                    "moveDown" => r = json!(n.move_down()),
                    "moveToBottom" => r = json!(n.move_to_bottom()),
                    "setZIndex" => n.set_z_index(arg.as_i64().unwrap()),
                    "remove" => n.remove(),
                    "destroy" => n.destroy(),
                    "show" => n.set_attr("visible", true.into()),
                    "hide" => n.set_attr("visible", false.into()),
                    "moveTo" => {
                        let to = handles.borrow().node(arg);
                        n.move_to(&to);
                    }
                    "move" => n.move_by(arg["x"].as_f64(), arg["y"].as_f64()),
                    "rotate" => n.rotate(f(arg)),
                    "setAbsolutePosition" => n.set_absolute_position(Vector2d {
                        x: f(&arg["x"]),
                        y: f(&arg["y"]),
                    }),
                    other => panic!("unknown method {other}"),
                }
            }
            "on" => {
                let label = op[3].as_str().unwrap().to_string();
                let cancel = op[4].as_bool().unwrap();
                let (log, handles) = (log.clone(), handles.clone());
                node().on(
                    op[2].as_str().unwrap(),
                    Rc::new(move |_: &Node, evt: &mut KonvaEvent| {
                        let handles = handles.borrow();
                        let mut entry = json!({
                            "l": label,
                            "type": evt.event_type,
                            "target": handles.name(evt.target.as_ref()),
                            "current": handles.name(evt.current_target.as_ref()),
                        });
                        // Only change events carry values; their old value
                        // is undefined when the attribute was unset.
                        if let Some(new) = &evt.new_val {
                            entry["old"] = evt.old_val.as_ref().map_or(Json::Null, value);
                            entry["new"] = value(new);
                        }
                        if let Some(child) = &evt.child {
                            entry["child"] = handles.name(Some(child));
                        }
                        log.borrow_mut().push(entry);
                        if cancel {
                            evt.cancel_bubble = true;
                        }
                    }),
                );
            }
            "off" => node().off(op[2].as_str()),
            "fire" => node().fire(
                op[2].as_str().unwrap(),
                &mut KonvaEvent::default(),
                op[3].as_bool().unwrap(),
            ),
            "get" => {
                r = query(
                    &handles.borrow(),
                    &node(),
                    op[2].as_str().unwrap(),
                    op.get(3),
                )
            }
            "draw" => node().draw(),
            "color" => {
                r = match color_to_rgba(op[2].as_str()) {
                    Some(c) => {
                        json!({ "r": num(c.r), "g": num(c.g), "b": num(c.b), "a": num(c.a) })
                    }
                    None => Json::Null,
                }
            }
            "tf" => r = run_transform(&op[1]),
            other => panic!("unknown op {other}"),
        }
        let mut out = json!({ "r": r });
        if !log.borrow().is_empty() {
            out["events"] = Json::Array(log.borrow().clone());
        }
        let mut calls = serde_json::Map::new();
        for (h, n) in &handles.borrow().with_canvases {
            let stage = n.class_name() == ClassName::Stage;
            for canvas in n.canvases().iter_mut() {
                let taken = canvas.context.take_calls();
                if !taken.is_empty() {
                    let name = format!("{h}.{}", canvas_name(canvas.kind, stage));
                    calls.insert(name, Json::Array(taken.iter().map(call).collect()));
                }
            }
        }
        if !calls.is_empty() {
            out["calls"] = Json::Object(calls);
        }
        results.push(out);
    }
    results
}

fn query(handles: &Handles, n: &Node, what: &str, arg: Option<&Json>) -> Json {
    let top = arg.filter(|a| a.is_string()).map(|a| handles.node(a));
    match what {
        "transform" => matrix(&n.transform()),
        "absoluteTransform" => matrix(&n.absolute_transform(top.as_ref())),
        "absolutePosition" => vector(n.absolute_position(top.as_ref())),
        "clientRect" => {
            let c = arg.unwrap();
            let flag = |k: &str| c[k].as_bool().unwrap_or(false);
            rect(n.client_rect(&ClientRectConfig {
                skip_transform: flag("skipTransform"),
                skip_shadow: flag("skipShadow"),
                skip_stroke: flag("skipStroke"),
                relative_to: c.get("relativeTo").map(|h| handles.node(h)),
            }))
        }
        "absoluteScale" => vector(n.absolute_scale()),
        "absoluteRotation" => num(n.absolute_rotation()),
        "decompose" => decomposed(n.absolute_transform(None).decompose()),
        "absoluteOpacity" => num(n.absolute_opacity()),
        "isVisible" => json!(n.is_visible()),
        "isListening" => json!(n.is_listening()),
        "zIndex" => json!(n.z_index()),
        "absoluteZIndex" => json!(n.absolute_z_index()),
        "depth" => json!(n.depth()),
        "attrs" => Json::Array(
            n.attrs()
                .iter()
                .map(|(k, v)| json!([k, value(v)]))
                .collect(),
        ),
        "children" => Json::Array(n.children().iter().map(|c| handles.name(Some(c))).collect()),
        "parent" => handles.name(n.parent().as_ref()),
        "layer" => handles.name(n.layer().as_ref()),
        "stage" => handles.name(n.stage().as_ref()),
        "position" => vector(n.position()),
        "size" => json!({ "width": num(n.width()), "height": num(n.height()) }),
        "colorKey" => n.color_key().map_or(Json::Null, |k| json!(k)),
        "hasFill" => json!(n.has_fill()),
        "hasStroke" => json!(n.has_stroke()),
        "hasHitStroke" => json!(n.has_hit_stroke()),
        "shadowRGBA" => n.shadow_rgba().map_or(Json::Null, |c| json!(c)),
        "useBufferCanvas" => json!(n.use_buffer_canvas()),
        "shouldDrawHit" => json!(n.should_draw_hit()),
        other => panic!("unknown query {other}"),
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
    max_ulps: u64,
    max_abs_diff: f64,
    others: usize,
    failures: Vec<String>,
    /// The allowed differences, by `<group> op <index><path>`.
    known: BTreeSet<String>,
    seen_known: Vec<String>,
    /// The op being compared, for messages.
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
                self.fail(ctx, format!("port {g:?}, Konva {w:?} ({d} ulps)"));
            } else {
                self.max_ulps = self.max_ulps.max(d);
                self.max_abs_diff = self.max_abs_diff.max((g - w).abs());
            }
            return;
        }
        match (got, want) {
            (Json::Array(g), Json::Array(w)) => {
                if g.len() != w.len() {
                    self.fail(ctx, format!("port {got}, Konva {want}"));
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
                    self.fail(ctx, format!("port {got}, Konva {want}"));
                    return;
                }
                for (k, w) in w {
                    self.compare(&format!("{ctx}.{k}"), &g[k], w);
                }
            }
            _ => {
                self.others += 1;
                if got != want {
                    self.fail(ctx, format!("port {got}, Konva {want}"));
                }
            }
        }
    }
}

#[test]
fn scene_graph_matches_konva_10_9_1() {
    let cases: Json = serde_json::from_str(include_str!("fixtures/cases.json")).unwrap();
    let oracle: Json = serde_json::from_str(include_str!("fixtures/konva-10.9.1.json")).unwrap();
    assert_eq!(oracle["konva"], "10.9.1");
    let groups = cases["groups"].as_array().unwrap();
    let recorded = oracle["groups"].as_array().unwrap();
    assert_eq!(groups.len(), recorded.len());
    let mut tally = Tally {
        known: include_str!("fixtures/konva-differences.txt")
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect(),
        ..Tally::default()
    };
    let (mut ops, mut events, mut calls) = (0, 0, 0);
    for (group, want) in groups.iter().zip(recorded) {
        let name = group["name"].as_str().unwrap();
        assert_eq!(want["name"], name);
        let got = run_group(group["seed"].as_i64().unwrap() as i32, &group["ops"]);
        let want = want["results"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{name}");
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            ops += 1;
            events += w["events"].as_array().map_or(0, |e| e.len());
            if let Some(by_canvas) = w["calls"].as_object() {
                calls += by_canvas
                    .values()
                    .map(|c| c.as_array().unwrap().len())
                    .sum::<usize>();
            }
            tally.op = group["ops"][i].to_string();
            tally.compare(&format!("{name} op {i}"), g, w);
        }
    }
    println!(
        "{ops} ops, {events} events, {calls} context calls; {} of {} numbers identical, \
         the others within {} ulps (max abs diff {:e}) except {} listed; {} other values",
        tally.identical,
        tally.numbers,
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
        "the port differs from Konva 10.9.1"
    );
}

/// `js::hypot` against V8's `Math.hypot` on the seeded pairs in the
/// fixture: Konva sizes non-scaling strokes with it.
#[test]
fn hypot_matches_v8() {
    let oracle: Json = serde_json::from_str(include_str!("fixtures/konva-10.9.1.json")).unwrap();
    let pairs = oracle["hypot"].as_array().unwrap();
    let mut differ = Vec::new();
    for p in pairs {
        let (a, b, want) = (f(&p[0]), f(&p[1]), f(&p[2]));
        let got = js::hypot(&[a, b]);
        if got.to_bits() != want.to_bits() {
            differ.push(format!("hypot({a:e}, {b:e}): port {got:e}, V8 {want:e}"));
        }
    }
    println!(
        "{} of {} Math.hypot results identical",
        pairs.len() - differ.len(),
        pairs.len()
    );
    for d in differ.iter().take(10) {
        println!("  {d}");
    }
    assert!(differ.is_empty());
}

/// `js::number_to_string` against `String(x)` in Node on seeded numbers
/// of every magnitude: Konva writes shadow colors with it.
#[test]
fn number_to_string_matches_v8() {
    let oracle: Json = serde_json::from_str(include_str!("fixtures/konva-10.9.1.json")).unwrap();
    let pairs = oracle["numberStrings"].as_array().unwrap();
    let mut differ = Vec::new();
    for p in pairs {
        let (x, want) = (f(&p[0]), p[1].as_str().unwrap());
        let got = js::number_to_string(x);
        if got != want {
            differ.push(format!("{x:e}: port {got}, V8 {want}"));
        }
    }
    println!(
        "{} of {} Number#toString results identical",
        pairs.len() - differ.len(),
        pairs.len()
    );
    for d in differ.iter().take(10) {
        println!("  {d}");
    }
    assert!(differ.is_empty());
}
