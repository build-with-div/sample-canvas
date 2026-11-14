//! The Konva shell on Div compared with the same app on Konva 10.9.1 in
//! pinned Chromium.
//!
//! `fixtures/fidelity/shell-scenarios.json` scripts mouse and key input.
//! `tools/konva-reference/capture.mjs` replayed it on
//! `tools/konva-reference/shell.html` in Chrome for Testing 147 and wrote
//! `shell-chromium.json`: a snapshot after load and after every step. This
//! test replays the same input on the Div shell and takes the same
//! snapshot. Compared:
//!
//! - every element of the app, by name (its id, or its parent's name and
//!   its index): its `getBoundingClientRect` (bit for bit), its attributes
//!   and, for an element with no child elements, its text;
//! - the focused element and the selection;
//! - Konva's node tree: each node's class, depth and attributes;
//! - the 2D context calls each layer's scene and hit canvas received since
//!   the last snapshot: names and strings exactly, numbers within
//!   `MAX_ULPS` (V8's `sin` and `cos` are not fdlibm's; see
//!   docs/phase2-plan.md).
//!
//! The differences must be exactly those listed, value for value, in
//! `fixtures/fidelity/shell-div-differences.txt`, each under the gap that
//! owns it: a new difference fails, and so does a listed one that no longer
//! occurs.

use div::prelude::*;
use div::testing::TestApp;
use konva::{Arg, CanvasCall, Node, NodeType, Value};
use konva_shell::shell::{Shell, HEIGHT, WIDTH};
use serde_json::{json, Map, Value as Json};
use std::collections::BTreeMap;

const SCENARIOS: &str = include_str!("fixtures/fidelity/shell-scenarios.json");
const CHROMIUM: &str = include_str!("fixtures/fidelity/shell-chromium.json");
const KNOWN_DIFFERENCES: &str = include_str!("fixtures/fidelity/shell-div-differences.txt");

/// The attributes the reference page reports (`ATTRIBUTES` in shell.js).
const ATTRIBUTES: &[&str] = &[
    "role",
    "aria-label",
    "aria-pressed",
    "aria-selected",
    "title",
    "type",
    "class",
    "width",
    "height",
];

/// The reference page's `Math.random` (`mulberry32(1)` in shell.js).
const SEED: i32 = 1;

/// The largest difference allowed in a context call's number, in units in
/// the last place.
const MAX_ULPS: u64 = 2;

fn mulberry32(seed: i32) -> impl FnMut() -> f64 {
    let mut seed = seed as u32;
    move || {
        seed = seed.wrapping_add(0x6d2b_79f5);
        let mut t = (seed ^ (seed >> 15)).wrapping_mul(1 | seed);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::Number(n) => json!(n),
        Value::Bool(b) => json!(b),
        Value::String(s) => json!(s),
        Value::Array(v) => json!(v),
    }
}

fn arg_json(arg: &Arg) -> Json {
    match arg {
        Arg::Number(n) => json!(n),
        Arg::Str(s) => json!(s),
        Arg::Bool(b) => json!(b),
        Arg::Numbers(v) => json!(v),
    }
}

/// A call in the reference page's form: `[method, ...args]` or
/// `['=property', value]`.
fn call_json(call: &CanvasCall) -> Json {
    match call {
        CanvasCall::Call { method, args } => {
            let mut out = vec![json!(method)];
            out.extend(args.iter().map(arg_json));
            Json::Array(out)
        }
        CanvasCall::Set { property, value } => json!([format!("={property}"), arg_json(value)]),
    }
}

fn element_children(element: &Element) -> Vec<Element> {
    let mut children = Vec::new();
    let mut child = element.first_child().unwrap();
    while let Some(current) = child {
        child = current.next_sibling().unwrap();
        children.push(current);
    }
    children
}

fn walk_elements(element: &Element, name: String, out: &mut Vec<Json>) {
    // An element with no box has an all-zero DOMRect.
    let rect = element
        .bounding_rect()
        .unwrap()
        .map(|r| [r.x, r.y, r.width, r.height])
        .unwrap_or_default();
    let mut attributes = Map::new();
    for attribute in ATTRIBUTES {
        if let Some(value) = element.get_attribute(attribute).unwrap() {
            attributes.insert(attribute.to_string(), Json::String(value));
        }
    }
    let children = element_children(element);
    let mut entry = json!({
        "name": name,
        "rect": rect.map(f64::from),
        "attributes": attributes,
    });
    if children.is_empty() {
        entry["text"] = Json::String(element.text_content().unwrap());
    }
    out.push(entry);
    for (index, child) in children.iter().enumerate() {
        let child_name = match child.get_attribute("id").unwrap() {
            Some(id) if !id.is_empty() => id,
            _ => format!("{name}/{index}"),
        };
        walk_elements(child, child_name, out);
    }
}

fn walk_tree(node: &Node, depth: usize, out: &mut Vec<Json>) {
    let attrs: Map<String, Json> = node
        .attrs()
        .iter()
        .map(|(k, v)| (k.clone(), value_json(v)))
        .collect();
    out.push(json!({
        "depth": depth,
        "className": node.class_name().as_str(),
        "attrs": attrs,
    }));
    if matches!(node.node_type(), NodeType::Layer | NodeType::Group) {
        for child in node.children() {
            walk_tree(&child, depth + 1, out);
        }
    }
}

/// The page's `snapshot()`, read from Div. Takes the recorded calls.
fn snapshot(ui: &TestApp, shell: &Shell) -> Json {
    let mut elements = Vec::new();
    walk_elements(&shell.root(), "app".into(), &mut elements);
    let document = ui.document();
    let body = document.body();
    let focus = document
        .focused_element()
        .unwrap()
        .filter(|element| !element.is_same_node(&body).unwrap())
        .map(|element| match element.get_attribute("id").unwrap() {
            Some(id) => id,
            None => format!("{:?}", element.kind().unwrap()).to_lowercase(),
        });
    let stage = shell.stage();
    let mut tree = Vec::new();
    let mut traces = Map::new();
    for layer in stage.children() {
        walk_tree(&layer, 0, &mut tree);
        let id = match layer.attr("id") {
            Some(Value::String(id)) => id,
            other => panic!("a layer id, not {other:?}"),
        };
        let mut canvases = layer.canvases();
        for (canvas, kind) in canvases.iter_mut().zip(["scene", "hit"]) {
            let calls = canvas.context.take_calls();
            traces.insert(
                format!("{id}.{kind}"),
                Json::Array(calls.iter().map(call_json).collect()),
            );
        }
    }
    json!({
        "elements": elements,
        "focus": focus,
        "selected": shell.selected(),
        "tree": tree,
        "traces": traces,
    })
}

fn key(name: &str) -> Key {
    match name {
        "Escape" => Key::Escape,
        "Delete" => Key::Delete,
        "Backspace" => Key::Backspace,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        other => panic!("unknown key {other}"),
    }
}

fn mouse(ui: &mut TestApp, kind: &str, x: f64, y: f64) {
    let kind = match kind {
        "move" => MouseEventType::Move,
        "down" => MouseEventType::Down,
        "up" => MouseEventType::Up,
        other => panic!("unknown mouse step {other}"),
    };
    ui.document()
        .dispatch_mouse_event(kind, x as f32, y as f32, MouseButton::Left, Modifiers::NONE)
        .expect("the mouse event dispatches");
}

fn step(ui: &mut TestApp, entry: &Json) {
    if let Some(kind) = entry["mouse"].as_str() {
        mouse(
            ui,
            kind,
            entry["x"].as_f64().unwrap(),
            entry["y"].as_f64().unwrap(),
        );
        ui.run_until_idle();
    } else if let Some(point) = entry["click"].as_array() {
        let (x, y) = (point[0].as_f64().unwrap(), point[1].as_f64().unwrap());
        for kind in ["move", "down", "up"] {
            mouse(ui, kind, x, y);
        }
        ui.run_until_idle();
    } else if let Some(name) = entry["key"].as_str() {
        ui.press(key(name));
    } else {
        panic!("unknown step {entry}");
    }
}

/// Numbers compare by value, so that 300 and 300.0 are equal.
fn same(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        (Json::Array(x), Json::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Json::Object(x), Json::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Bitwise, so that an f32 box can't pass for a LayoutUnit one.
fn same_rect(a: &Json, b: &Json) -> bool {
    let (a, b) = (a.as_array().unwrap(), b.as_array().unwrap());
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            a.to_bits() == b.to_bits() || (a == 0.0 && b == 0.0)
        })
}

fn ulps(a: f64, b: f64) -> u64 {
    if a == b {
        0
    } else if a.is_nan() || b.is_nan() || a.signum() != b.signum() {
        u64::MAX
    } else {
        a.to_bits().abs_diff(b.to_bits())
    }
}

/// Trace numbers: identical, or within `MAX_ULPS`. Returns the largest
/// difference in ulps, or `None` if the structure or a string differs.
fn trace_ulps(a: &Json, b: &Json) -> Option<u64> {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => Some(ulps(x.as_f64()?, y.as_f64()?)),
        (Json::Array(x), Json::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .try_fold(0, |m, (a, b)| Some(m.max(trace_ulps(a, b)?))),
        _ => (a == b).then_some(0),
    }
}

#[derive(Default)]
struct Tally {
    checked: BTreeMap<&'static str, (usize, usize)>,
    differences: Vec<String>,
    calls: usize,
    numbers: usize,
    numbers_off: usize,
    max_ulps: u64,
}

impl Tally {
    fn check(&mut self, at: &str, field: &'static str, ok: bool, detail: impl FnOnce() -> String) {
        let entry = self.checked.entry(field).or_default();
        entry.1 += 1;
        if ok {
            entry.0 += 1;
        } else {
            self.differences.push(format!("{at} {field}: {}", detail()));
        }
    }

    /// Counts the numbers in a matching trace and how far off they are.
    fn count_numbers(&mut self, ours: &Json, theirs: &Json) {
        match (ours, theirs) {
            (Json::Number(x), Json::Number(y)) => {
                self.numbers += 1;
                let u = ulps(x.as_f64().unwrap(), y.as_f64().unwrap());
                if u > 0 {
                    self.numbers_off += 1;
                    self.max_ulps = self.max_ulps.max(u);
                }
            }
            (Json::Array(x), Json::Array(y)) => {
                for (a, b) in x.iter().zip(y) {
                    self.count_numbers(a, b);
                }
            }
            _ => {}
        }
    }
}

fn compare(tally: &mut Tally, at: &str, div: &Json, chromium: &Json) {
    let names = |s: &Json| -> Vec<String> {
        s["elements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect()
    };
    let (div_names, chromium_names) = (names(div), names(chromium));
    tally.check(at, "elements", div_names == chromium_names, || {
        format!("Div {div_names:?}, Chromium {chromium_names:?}")
    });
    for (ours, theirs) in div["elements"]
        .as_array()
        .unwrap()
        .iter()
        .zip(chromium["elements"].as_array().unwrap())
    {
        let name = theirs["name"].as_str().unwrap();
        tally.check(at, "box", same_rect(&ours["rect"], &theirs["rect"]), || {
            format!("{name}: Div {}, Chromium {}", ours["rect"], theirs["rect"])
        });
        tally.check(
            at,
            "attributes",
            ours["attributes"] == theirs["attributes"],
            || {
                format!(
                    "{name}: Div {}, Chromium {}",
                    ours["attributes"], theirs["attributes"]
                )
            },
        );
        tally.check(at, "text", ours.get("text") == theirs.get("text"), || {
            format!(
                "{name}: Div {}, Chromium {}",
                ours.get("text").unwrap_or(&Json::Null),
                theirs.get("text").unwrap_or(&Json::Null)
            )
        });
    }
    for field in ["focus", "selected"] {
        let name: &'static str = if field == "focus" {
            "focus"
        } else {
            "selected"
        };
        tally.check(at, name, div[field] == chromium[field], || {
            format!("Div {}, Chromium {}", div[field], chromium[field])
        });
    }
    tally.check(at, "tree", same(&div["tree"], &chromium["tree"]), || {
        format!("Div {}, Chromium {}", div["tree"], chromium["tree"])
    });
    let (ours, theirs) = (
        div["traces"].as_object().unwrap(),
        chromium["traces"].as_object().unwrap(),
    );
    let (our_keys, their_keys): (Vec<_>, Vec<_>) = (ours.keys().collect(), theirs.keys().collect());
    tally.check(at, "canvases", our_keys == their_keys, || {
        format!("Div {our_keys:?}, Chromium {their_keys:?}")
    });
    for (canvas, want) in theirs {
        let got = ours.get(canvas).unwrap_or(&Json::Null);
        let u = trace_ulps(got, want);
        if u.is_some() {
            tally.calls += want.as_array().unwrap().len();
            tally.count_numbers(got, want);
        }
        tally.check(at, "trace", u.is_some_and(|u| u <= MAX_ULPS), || {
            let (d, c) = (
                got.as_array().cloned().unwrap_or_default(),
                want.as_array().unwrap().clone(),
            );
            let first = d
                .iter()
                .zip(&c)
                .position(|(a, b)| trace_ulps(a, b).is_none_or(|u| u > MAX_ULPS))
                .unwrap_or(d.len().min(c.len()));
            format!(
                "{canvas}: {} calls vs {}; first difference at {first}: Div {}, Chromium {}",
                d.len(),
                c.len(),
                d.get(first).unwrap_or(&Json::Null),
                c.get(first).unwrap_or(&Json::Null)
            )
        });
    }
}

#[test]
fn shell_matches_konva_in_chromium() {
    let spec: Json = serde_json::from_str(SCENARIOS).unwrap();
    let chromium: Json = serde_json::from_str(CHROMIUM).unwrap();
    assert_eq!(spec["host"]["width"].as_f64(), Some(f64::from(WIDTH)));
    assert_eq!(spec["host"]["height"].as_f64(), Some(f64::from(HEIGHT)));
    let mut tally = Tally::default();
    let mut snapshots = 0;
    for scenario in spec["scenarios"].as_array().unwrap() {
        let id = scenario["id"].as_str().unwrap();
        let reference = chromium["scenarios"][id]
            .as_array()
            .expect("Chromium captured the scenario");
        // As the page does: Konva's hit colors come from a seeded
        // Math.random, and no other shape holds a color yet.
        konva::clear_color_keys();
        konva::set_math_random(mulberry32(SEED));
        let mut ui = TestApp::new(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
            .mount(|| view! { <div id="host"></div> });
        let host = ui.by_id("host");
        let shell = Shell::mount(&ui.document().clone(), &host);
        ui.run_until_idle();
        let steps = scenario["steps"].as_array().unwrap();
        assert_eq!(
            reference.len(),
            steps.len() + 1,
            "{id}: one snapshot per step"
        );
        for (index, want) in reference.iter().enumerate() {
            if index > 0 {
                step(&mut ui, &steps[index - 1]);
            }
            let got = snapshot(&ui, &shell);
            compare(&mut tally, &format!("{id}#{index}"), &got, want);
            snapshots += 1;
        }
    }

    let browser = chromium["chromium"].as_str().unwrap();
    for (field, (matched, total)) in &tally.checked {
        println!("FIDELITY konva-shell {field}: {matched}/{total} match {browser}");
    }
    println!(
        "FIDELITY konva-shell: {snapshots} snapshots, {} differences; {} context calls in matching traces, {} of {} numbers not bit-identical (max {} ulps)",
        tally.differences.len(),
        tally.calls,
        tally.numbers_off,
        tally.numbers,
        tally.max_ulps
    );

    // The list: `## <gap>` headings, then one difference per line.
    let mut listed = Vec::new();
    let mut gap = None;
    let mut per_gap: BTreeMap<String, usize> = BTreeMap::new();
    for line in KNOWN_DIFFERENCES.lines().map(str::trim_end) {
        if let Some(heading) = line.strip_prefix("## ") {
            gap = Some(heading.split_whitespace().next().unwrap().to_owned());
        } else if !line.is_empty() && !line.starts_with('#') {
            let gap = gap
                .clone()
                .expect("every difference is under a gap heading");
            *per_gap.entry(gap).or_default() += 1;
            listed.push(line.to_owned());
        }
    }
    for (gap, count) in &per_gap {
        println!("FIDELITY konva-shell differences owned by {gap}: {count}");
    }
    let unexplained: Vec<&String> = tally
        .differences
        .iter()
        .filter(|line| !listed.contains(line))
        .collect();
    let resolved: Vec<&String> = listed
        .iter()
        .filter(|line| !tally.differences.contains(line))
        .collect();
    for line in &unexplained {
        println!("DIFF (not listed) {line}");
    }
    for line in &resolved {
        println!("DIFF (listed, no longer occurs) {line}");
    }
    assert!(
        unexplained.is_empty() && resolved.is_empty(),
        "{} differences are not in shell-div-differences.txt and {} listed ones no longer occur",
        unexplained.len(),
        resolved.len()
    );
}
