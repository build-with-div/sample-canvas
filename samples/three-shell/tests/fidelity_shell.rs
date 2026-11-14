//! The three.js shell on Div compared with the same page on three.js 0.187.0
//! in pinned Chromium.
//!
//! `fixtures/fidelity/shell-scenarios.json` scripts mouse, wheel and key
//! input. `tools/three-reference/capture.mjs` replayed it on
//! `tools/three-reference/shell.html` in Chrome for Testing 147 and wrote
//! `shell-chromium.json`: a snapshot after load and after every step (and
//! the animation frame that follows it). This test replays the same input
//! on the Div shell and takes the same snapshot. Compared:
//!
//! - every element of the app, by name (its id, or its parent's name and
//!   its index): its `getBoundingClientRect` (bit for bit), its attributes
//!   and, for an element with no child elements, its text;
//! - the focused element and the selected object;
//! - the camera (position, quaternion, aspect, zoom, projection matrix) and
//!   the controls (target, state, options): numbers within `MAX_ULPS`,
//!   everything else exactly;
//! - the controls' `change`, `start` and `end` events since the last
//!   snapshot, in order.
//!
//! The differences must be exactly those listed in
//! `fixtures/fidelity/shell-div-differences.txt`, each under the gap that
//! owns it: a new difference fails, and so does a listed one that no longer
//! occurs.

use div::prelude::*;
use div::testing::TestApp;
use serde_json::{json, Map, Value as Json};
use std::collections::BTreeMap;
use three_shell::shell::{Shell, HEIGHT, WIDTH};

const SCENARIOS: &str = include_str!("fixtures/fidelity/shell-scenarios.json");
const CHROMIUM: &str = include_str!("fixtures/fidelity/shell-chromium.json");
const KNOWN_DIFFERENCES: &str = include_str!("fixtures/fidelity/shell-div-differences.txt");

/// The attributes the reference page reports (`ATTRIBUTES` in shell.js).
const ATTRIBUTES: &[&str] = &[
    "role",
    "aria-label",
    "aria-pressed",
    "aria-selected",
    "aria-expanded",
    "aria-orientation",
    "aria-valuenow",
    "title",
    "type",
    "width",
    "height",
];

/// The largest difference allowed in a camera or controls number, in units
/// in the last place.
const MAX_ULPS: u64 = 2;

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

/// The page's `snapshot()`, read from Div. Takes the controls' events.
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
    let events = shell.take_events();
    let controls = shell.controls();
    let camera = &controls.camera;
    let q = camera.quaternion();
    json!({
        "elements": elements,
        "focus": focus,
        "selected": shell.selected(),
        "camera": {
            "position": [camera.position.x, camera.position.y, camera.position.z],
            "quaternion": [q.x, q.y, q.z, q.w],
            "aspect": camera.aspect,
            "zoom": camera.zoom,
            "projection": camera.projection_matrix.elements.to_vec(),
        },
        "controls": {
            "target": [controls.target.x, controls.target.y, controls.target.z],
            "state": controls.state.code(),
            "autoRotate": controls.auto_rotate,
            "enableDamping": controls.enable_damping,
            "rotateSpeed": controls.rotate_speed,
            "screenSpacePanning": controls.screen_space_panning,
        },
        "events": events,
    })
}

fn key(name: &str) -> Key {
    match name {
        "Escape" => Key::Escape,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        other => panic!("unknown key {other}"),
    }
}

fn modifiers(entry: &Json) -> Modifiers {
    let mut bits = 0;
    for name in entry["modifiers"].as_array().into_iter().flatten() {
        bits |= match name.as_str().unwrap() {
            "shift" => Modifiers::SHIFT.bits(),
            "ctrl" => Modifiers::CTRL.bits(),
            "alt" => Modifiers::ALT.bits(),
            "meta" => Modifiers::META.bits(),
            other => panic!("unknown modifier {other}"),
        };
    }
    Modifiers(bits)
}

fn button(name: Option<&str>) -> MouseButton {
    match name.unwrap_or("left") {
        "left" => MouseButton::Left,
        "middle" => MouseButton::Middle,
        "right" => MouseButton::Right,
        other => panic!("unknown button {other}"),
    }
}

fn mouse(ui: &mut TestApp, kind: &str, x: f64, y: f64, button: MouseButton, modifiers: Modifiers) {
    let kind = match kind {
        "move" => MouseEventType::Move,
        "down" => MouseEventType::Down,
        "up" => MouseEventType::Up,
        other => panic!("unknown mouse step {other}"),
    };
    ui.document()
        .dispatch_mouse_event(kind, x as f32, y as f32, button, modifiers)
        .expect("the mouse event dispatches");
}

fn step(ui: &mut TestApp, entry: &Json) {
    if let Some(kind) = entry["mouse"].as_str() {
        mouse(
            ui,
            kind,
            entry["x"].as_f64().unwrap(),
            entry["y"].as_f64().unwrap(),
            button(entry["button"].as_str()),
            modifiers(entry),
        );
    } else if let Some(point) = entry["click"].as_array() {
        let (x, y) = (point[0].as_f64().unwrap(), point[1].as_f64().unwrap());
        for kind in ["move", "down", "up"] {
            mouse(ui, kind, x, y, MouseButton::Left, Modifiers::NONE);
        }
    } else if let Some(wheel) = entry["wheel"].as_array() {
        let v: Vec<f32> = wheel.iter().map(|v| v.as_f64().unwrap() as f32).collect();
        ui.document()
            .dispatch_wheel_event(v[0], v[1], 0.0, v[2], modifiers(entry))
            .expect("the wheel event dispatches");
    } else if let Some(name) = entry["key"].as_str() {
        ui.press_with(modifiers(entry), key(name));
    } else if entry["idle"].as_bool() != Some(true) {
        panic!("unknown step {entry}");
    }
    ui.run_until_idle();
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

/// Units in the last place between two numbers; +0 and -0 are equal (the
/// page's JSON writes -0 as 0).
fn ulps(a: f64, b: f64) -> u64 {
    if a == b {
        0
    } else if a.is_nan() || b.is_nan() || a.signum() != b.signum() {
        u64::MAX
    } else {
        a.to_bits().abs_diff(b.to_bits())
    }
}

#[derive(Default)]
struct Tally {
    checked: BTreeMap<&'static str, (usize, usize)>,
    differences: Vec<String>,
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

    /// The camera and controls: numbers within `MAX_ULPS`, the rest
    /// exactly, value by value.
    fn values(&mut self, at: &str, field: &'static str, path: &str, ours: &Json, theirs: &Json) {
        match (ours, theirs) {
            (Json::Number(x), Json::Number(y)) => {
                let u = ulps(x.as_f64().unwrap(), y.as_f64().unwrap());
                self.numbers += 1;
                if u > 0 && u <= MAX_ULPS {
                    self.numbers_off += 1;
                    self.max_ulps = self.max_ulps.max(u);
                }
                self.check(at, field, u <= MAX_ULPS, || {
                    format!("{path}: Div {ours}, Chromium {theirs}")
                });
            }
            (Json::Array(x), Json::Array(y)) if x.len() == y.len() => {
                for (index, (a, b)) in x.iter().zip(y).enumerate() {
                    self.values(at, field, &format!("{path}[{index}]"), a, b);
                }
            }
            (Json::Object(x), Json::Object(y))
                if x.keys().collect::<Vec<_>>() == y.keys().collect::<Vec<_>>() =>
            {
                for (key, b) in y {
                    self.values(at, field, &format!("{path}.{key}"), &x[key], b);
                }
            }
            _ => self.check(at, field, ours == theirs, || {
                format!("{path}: Div {ours}, Chromium {theirs}")
            }),
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
    tally.check(at, "focus", div["focus"] == chromium["focus"], || {
        format!("Div {}, Chromium {}", div["focus"], chromium["focus"])
    });
    tally.check(
        at,
        "selected",
        div["selected"] == chromium["selected"],
        || format!("Div {}, Chromium {}", div["selected"], chromium["selected"]),
    );
    tally.check(at, "events", div["events"] == chromium["events"], || {
        format!("Div {}, Chromium {}", div["events"], chromium["events"])
    });
    tally.values(at, "camera", "camera", &div["camera"], &chromium["camera"]);
    tally.values(
        at,
        "controls",
        "controls",
        &div["controls"],
        &chromium["controls"],
    );
}

#[test]
fn shell_matches_three_in_chromium() {
    let spec: Json = serde_json::from_str(SCENARIOS).unwrap();
    let chromium: Json = serde_json::from_str(CHROMIUM).unwrap();
    assert_eq!(spec["host"]["width"].as_f64(), Some(f64::from(WIDTH)));
    assert_eq!(spec["host"]["height"].as_f64(), Some(f64::from(HEIGHT)));
    assert_eq!(chromium["three"], "0.187.0");
    let mut tally = Tally::default();
    let (mut snapshots, mut events) = (0, 0);
    for scenario in spec["scenarios"].as_array().unwrap() {
        let id = scenario["id"].as_str().unwrap();
        let reference = chromium["scenarios"][id]
            .as_array()
            .expect("Chromium captured the scenario");
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
                // The animation frame the capture runs after each step.
                shell.frame();
            }
            let got = snapshot(&ui, &shell);
            compare(&mut tally, &format!("{id}#{index}"), &got, want);
            events += want["events"].as_array().unwrap().len();
            snapshots += 1;
        }
    }

    let browser = chromium["chromium"].as_str().unwrap();
    for (field, (matched, total)) in &tally.checked {
        println!("FIDELITY three-shell {field}: {matched}/{total} match {browser}");
    }
    println!(
        "FIDELITY three-shell: {snapshots} snapshots, {events} controls events, {} differences; {} of {} camera and controls numbers not bit-identical but within {MAX_ULPS} ulps (max {})",
        tally.differences.len(),
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
        println!("FIDELITY three-shell differences owned by {gap}: {count}");
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
