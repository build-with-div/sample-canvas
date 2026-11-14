//! The board on Div compared with the same board as a web page in pinned
//! Chromium.
//!
//! `fixtures/fidelity/board-scenarios.json` scripts mouse and key input.
//! `tools/board-reference/capture.mjs` replayed it on
//! `tools/board-reference/board.html` in Chrome for Testing 147 and wrote
//! `board-chromium.json`: a snapshot after load and after every step. This
//! test replays the same input on the Div shell and takes the same
//! snapshot. Compared:
//!
//! - every element with an id: its `getBoundingClientRect` (bit for bit)
//!   and its attributes;
//! - the focused element;
//! - the tool, the selection and the scene's elements;
//! - the 2D context calls of the last redraw: names and strings exactly,
//!   numbers within `MAX_ULPS` (the V8 `sin`/`cos` difference documented
//!   in docs/phase2-plan.md).
//!
//! The differences must be exactly those listed, value for value, in
//! `fixtures/fidelity/board-div-differences.txt`, each under the gap that
//! owns it: a new difference fails, and so does a listed one that no longer
//! occurs.

use board::shell::{Board, HEIGHT, WIDTH};
use board::Scene;
use div::prelude::*;
use div::testing::TestApp;
use roughjs::CanvasCall;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const SCENARIOS: &str = include_str!("fixtures/fidelity/board-scenarios.json");
const CHROMIUM: &str = include_str!("fixtures/fidelity/board-chromium.json");
const KNOWN_DIFFERENCES: &str = include_str!("fixtures/fidelity/board-div-differences.txt");

/// The attributes the reference page reports (`ATTRIBUTES` in board.js).
const ATTRIBUTES: &[&str] = &[
    "role",
    "aria-label",
    "aria-pressed",
    "aria-selected",
    "data-tool",
    "data-section",
    "data-value",
    "title",
    "type",
];

/// The largest difference allowed in a context call's number, in units in
/// the last place.
const MAX_ULPS: u64 = 2;

fn scene_json(scene: &Scene) -> Value {
    Value::Array(
        scene
            .elements
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "type": e.kind,
                    "name": e.name,
                    "x": e.x,
                    "y": e.y,
                    "width": e.width,
                    "height": e.height,
                    "strokeColor": e.style.stroke_color,
                    "backgroundColor": e.style.background_color,
                    "strokeWidth": e.style.stroke_width,
                    "roughness": e.style.roughness,
                    "fillStyle": e.style.fill_style,
                    "seed": e.seed,
                })
            })
            .collect(),
    )
}

/// A call in the reference page's form: `[name, ...args]`.
fn call_json(call: &CanvasCall) -> Value {
    match call {
        CanvasCall::Save => json!(["save"]),
        CanvasCall::Restore => json!(["restore"]),
        CanvasCall::SetStrokeStyle(s) => json!(["strokeStyle=", s]),
        CanvasCall::SetFillStyle(s) => json!(["fillStyle=", s]),
        CanvasCall::SetLineWidth(v) => json!(["lineWidth=", v]),
        CanvasCall::SetLineDash(v) => json!(["setLineDash", v]),
        CanvasCall::SetLineDashOffset(v) => json!(["lineDashOffset=", v]),
        CanvasCall::BeginPath => json!(["beginPath"]),
        CanvasCall::MoveTo(x, y) => json!(["moveTo", x, y]),
        CanvasCall::LineTo(x, y) => json!(["lineTo", x, y]),
        CanvasCall::BezierCurveTo(d) => {
            json!(["bezierCurveTo", d[0], d[1], d[2], d[3], d[4], d[5]])
        }
        CanvasCall::Fill(rule) => json!(["fill", rule.as_str()]),
        CanvasCall::Stroke => json!(["stroke"]),
    }
}

/// The page's `snapshot()`, read from Div.
fn snapshot(ui: &TestApp, board: &Board) -> Value {
    let mut elements = Vec::new();
    let mut stack = vec![board.root.clone()];
    // Document order: depth first, children in order.
    while let Some(element) = stack.pop() {
        let mut children = Vec::new();
        let mut child = element.first_child().unwrap();
        while let Some(current) = child {
            child = current.next_sibling().unwrap();
            children.push(current);
        }
        stack.extend(children.into_iter().rev());
        let Some(id) = element.get_attribute("id").unwrap() else {
            continue;
        };
        // An element with no box has an all-zero DOMRect.
        let rect = element
            .bounding_rect()
            .unwrap()
            .map(|r| [r.x, r.y, r.width, r.height])
            .unwrap_or_default();
        let mut attributes = Map::new();
        for name in ATTRIBUTES {
            if let Some(value) = element.get_attribute(name).unwrap() {
                attributes.insert(name.to_string(), Value::String(value));
            }
        }
        elements.push(json!({
            "id": id,
            "rect": rect.map(f64::from),
            "attributes": attributes,
        }));
    }
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
    let scene = board.scene.get();
    json!({
        "elements": elements,
        "focus": focus,
        "tool": scene.tool.name(),
        "selected": scene.selected,
        "scene": scene_json(&scene),
        "trace": board.trace.borrow().iter().map(call_json).collect::<Vec<_>>(),
    })
}

fn key(name: &str) -> Key {
    match name {
        "Escape" => Key::Escape,
        "Delete" => Key::Delete,
        "Backspace" => Key::Backspace,
        other if other.chars().count() == 1 => Key::Char(other.chars().next().unwrap()),
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

fn step(ui: &mut TestApp, entry: &Value) {
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
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Bitwise, so that an f32 box can't pass for a LayoutUnit one.
fn same_rect(a: &Value, b: &Value) -> bool {
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
fn trace_ulps(a: &Value, b: &Value) -> Option<u64> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Some(ulps(x.as_f64()?, y.as_f64()?)),
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
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
    trace_numbers_off: usize,
    trace_max_ulps: u64,
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
}

fn compare(tally: &mut Tally, at: &str, div: &Value, chromium: &Value) {
    let ids = |s: &Value| -> Vec<String> {
        s["elements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let (div_ids, chromium_ids) = (ids(div), ids(chromium));
    tally.check(at, "elements", div_ids == chromium_ids, || {
        format!("Div {div_ids:?}, Chromium {chromium_ids:?}")
    });
    for (ours, theirs) in div["elements"]
        .as_array()
        .unwrap()
        .iter()
        .zip(chromium["elements"].as_array().unwrap())
    {
        let id = theirs["id"].as_str().unwrap();
        tally.check(at, "box", same_rect(&ours["rect"], &theirs["rect"]), || {
            format!("{id}: Div {}, Chromium {}", ours["rect"], theirs["rect"])
        });
        tally.check(
            at,
            "attributes",
            ours["attributes"] == theirs["attributes"],
            || {
                format!(
                    "{id}: Div {}, Chromium {}",
                    ours["attributes"], theirs["attributes"]
                )
            },
        );
    }
    for field in ["focus", "tool", "selected", "scene"] {
        let name: &'static str = match field {
            "focus" => "focus",
            "tool" => "tool",
            "selected" => "selected",
            _ => "scene",
        };
        tally.check(at, name, same(&div[field], &chromium[field]), || {
            format!("Div {}, Chromium {}", div[field], chromium[field])
        });
    }
    let ulps = trace_ulps(&div["trace"], &chromium["trace"]);
    if let Some(u) = ulps {
        if u > 0 {
            tally.trace_numbers_off += 1;
            tally.trace_max_ulps = tally.trace_max_ulps.max(u);
        }
    }
    tally.check(at, "trace", ulps.is_some_and(|u| u <= MAX_ULPS), || {
        let (d, c) = (
            div["trace"].as_array().unwrap(),
            chromium["trace"].as_array().unwrap(),
        );
        let first = d
            .iter()
            .zip(c)
            .position(|(a, b)| trace_ulps(a, b).is_none_or(|u| u > MAX_ULPS));
        format!(
            "{} calls vs {}; first difference at {first:?}",
            d.len(),
            c.len()
        )
    });
}

#[test]
fn board_matches_the_reference_page_in_chromium() {
    let spec: Value = serde_json::from_str(SCENARIOS).unwrap();
    let chromium: Value = serde_json::from_str(CHROMIUM).unwrap();
    assert_eq!(spec["host"]["width"].as_f64(), Some(f64::from(WIDTH)));
    assert_eq!(spec["host"]["height"].as_f64(), Some(f64::from(HEIGHT)));
    let mut tally = Tally::default();
    let mut snapshots = 0;
    for scenario in spec["scenarios"].as_array().unwrap() {
        let id = scenario["id"].as_str().unwrap();
        let reference = chromium["scenarios"][id]
            .as_array()
            .expect("Chromium captured the scenario");
        let mut ui = TestApp::new(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
            .mount(|| view! { <div id="host"></div> });
        let host = ui.by_id("host");
        let board = Board::mount(&ui.document().clone(), &host);
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
            let got = snapshot(&ui, &board);
            compare(&mut tally, &format!("{id}#{index}"), &got, want);
            snapshots += 1;
        }
    }

    let browser = chromium["chromium"].as_str().unwrap();
    for (field, (matched, total)) in &tally.checked {
        println!("FIDELITY board {field}: {matched}/{total} match {browser}");
    }
    println!(
        "FIDELITY board: {snapshots} snapshots, {} differences; {} traces not bit-identical (max {} ulps)",
        tally.differences.len(),
        tally.trace_numbers_off,
        tally.trace_max_ulps
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
        println!("FIDELITY board differences owned by {gap}: {count}");
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
        "{} differences are not in board-div-differences.txt and {} listed ones no longer occur",
        unexplained.len(),
        resolved.len()
    );
}
