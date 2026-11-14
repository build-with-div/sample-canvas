//! The Konva demo shell on Div: a toolbar, a stage in its container, a
//! layers panel and an inspector, over the Konva port in `samples/konva`.
//!
//! The DOM, its inline styles and the app code are those of the reference
//! page, `tools/konva-reference/shell.js`, element for element and step for
//! step, so that `tests/fidelity_shell.rs` can compare the two in pinned
//! Chromium.

use crate::stage_dom::StageDom;
use div::prelude::*;
use div::typed_style::{Color, Display, Style, StyleColor};
use konva::{js, ClassName, Node, NodeType, Value};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The shell's size in CSS pixels.
pub const WIDTH: f32 = 960.0;
pub const HEIGHT: f32 = 600.0;
/// The stage's size.
pub const STAGE_WIDTH: f64 = 480.0;
pub const STAGE_HEIGHT: f64 = 360.0;

const PRESSED: u32 = 0xe0dfff;
const SELECTED_SWATCH: u32 = 0x4a47b1;
const SWATCH_BORDER: u32 = 0xcccccc;
const WHITE: u32 = 0xffffff;
const FILLS: [&str; 4] = ["#e03131", "#1971c2", "#2f9e44", "#f08c00"];
/// (attribute, label, step)
const FIELDS: [(&str, &str, f64); 4] = [
    ("x", "x", 10.0),
    ("y", "y", 10.0),
    ("rotation", "Rotation", 15.0),
    ("opacity", "Opacity", 0.1),
];
const TOOLS: [(&str, &str); 5] = [
    ("add-rect", "Rect"),
    ("add-circle", "Circle"),
    ("raise", "Up"),
    ("lower", "Down"),
    ("delete", "Delete"),
];

/// A mounted shell.
#[derive(Clone)]
pub struct Shell(Rc<Inner>);

struct Inner {
    document: Document,
    root: Element,
    stage: StageDom,
    shapes: Node,
    layers_panel: Element,
    inspector: Element,
    title: Element,
    values: Vec<Element>,
    fill_row: Element,
    swatches: Vec<(Element, &'static str)>,
    visible_button: Element,
    selected: RefCell<Option<String>>,
    created: Cell<u32>,
}

fn color(hex: u32) -> Color {
    Color::from_hex(hex, false)
}

fn hex_color(value: &str) -> Color {
    color(u32::from_str_radix(value.trim_start_matches('#'), 16).expect("a hex color"))
}

fn set_border_color(element: &Element, hex: u32) {
    let c = StyleColor::Resolved(color(hex));
    element.set_border_top_color(c).unwrap();
    element.set_border_right_color(c).unwrap();
    element.set_border_bottom_color(c).unwrap();
    element.set_border_left_color(c).unwrap();
}

// div-gap: 15 CSS border shorthands. Div's `style!` rejects `border-width`,
// `border-style` and `border-color`, and takes a border width only as a
// unitless integer. So the shell's styles spell out the twelve longhands,
// with `1` for `1px`, where the reference page uses the three shorthands.
// The computed style is the same.
fn el(document: &Document, tag: &str, style: &Style, attributes: &[(&str, &str)]) -> Element {
    let element = Element::create(document, tag).expect("a known tag");
    element.apply_style(style).expect("the style applies");
    for (name, value) in attributes {
        element.set_attribute(name, value).expect("an attribute");
    }
    element
}

fn with_text(element: Element, text: &str) -> Element {
    if !text.is_empty() {
        element.set_text(text).expect("text");
    }
    element
}

fn row_style(depth: usize) -> Style {
    match depth {
        0 => style!("box-sizing: border-box; height: 24px; padding-left: 8px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
        1 => style!("box-sizing: border-box; height: 24px; padding-left: 24px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
        _ => style!("box-sizing: border-box; height: 24px; padding-left: 40px; padding-right: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
    }
}

fn step_button(document: &Document, attributes: &[(&str, &str)], text: &str) -> Element {
    with_text(
        el(
            document,
            "button",
            &style!("box-sizing: border-box; width: 24px; height: 24px; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
            attributes,
        ),
        text,
    )
}

fn string_attr(node: &Node, key: &str) -> String {
    match node.attr(key) {
        Some(Value::String(s)) => s,
        _ => String::new(),
    }
}

fn is_container(node: &Node) -> bool {
    matches!(node.node_type(), NodeType::Layer | NodeType::Group)
}

fn children(node: &Node) -> Vec<Node> {
    if is_container(node) {
        node.children()
    } else {
        Vec::new()
    }
}

/// The node with this `id`, depth first, as Konva orders the tree.
fn node_by_id(nodes: &[Node], id: &str) -> Option<Node> {
    for node in nodes {
        if string_attr(node, "id") == id {
            return Some(node.clone());
        }
        if let Some(found) = node_by_id(&children(node), id) {
            return Some(found);
        }
    }
    None
}

fn number(node: &Node, attr: &str) -> f64 {
    match attr {
        "x" => node.x(),
        "y" => node.y(),
        "rotation" => node.rotation(),
        "opacity" => node.opacity(),
        other => panic!("no field {other}"),
    }
}

/// `layer.batchDraw()`.
///
/// div-gap: W11 requestAnimationFrame. Konva's `batchDraw` draws the layer
/// in the next animation frame; Div has no frame callback, so the shell
/// draws it at once, where the reference page batches it.
fn redraw(layer: &Node) {
    layer.draw();
}

/// Runs `f` on the shell from an event listener, unless it is gone.
fn listener(inner: &Rc<Inner>, f: impl Fn(&Rc<Inner>) + 'static) -> impl Fn(&Event) + 'static {
    let weak: Weak<Inner> = Rc::downgrade(inner);
    move |_: &Event| {
        if let Some(inner) = weak.upgrade() {
            f(&inner);
        }
    }
}

impl Shell {
    /// Builds the shell and appends it to `parent`.
    pub fn mount(document: &Document, parent: &Element) -> Shell {
        with_document(document, || Self::build(document, parent))
    }

    /// The shell's root element, `#app`.
    pub fn root(&self) -> Element {
        self.0.root.clone()
    }

    /// The stage.
    pub fn stage(&self) -> Node {
        self.0.stage.stage.clone()
    }

    /// The selected node's id.
    pub fn selected(&self) -> Option<String> {
        self.0.selected.borrow().clone()
    }

    fn build(document: &Document, parent: &Element) -> Shell {
        let root = el(
            document,
            "div",
            &style!("position: relative; width: 960px; height: 600px; overflow: hidden; background-color: #f8f9fa; font-family: sans-serif; font-size: 12px; line-height: 16px; color: #1e1e1e"),
            &[("id", "app")],
        );

        let toolbar = el(
            document,
            "div",
            &style!("position: absolute; left: 232px; top: 16px; display: flex; column-gap: 4px; height: 28px"),
            &[("id", "toolbar"), ("role", "toolbar"), ("aria-label", "Edit")],
        );
        let mut tool_buttons = Vec::new();
        for (id, label) in TOOLS {
            let button = with_text(
                el(
                    document,
                    "button",
                    &style!("box-sizing: border-box; width: 56px; height: 28px; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
                    &[("id", id), ("type", "button")],
                ),
                label,
            );
            toolbar.append_child(&button).unwrap();
            tool_buttons.push((button, id));
        }
        root.append_child(&toolbar).unwrap();

        let container = el(
            document,
            "div",
            &style!("position: absolute; left: 232px; top: 56px; width: 480px; height: 360px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; background-color: #ffffff"),
            &[("id", "stage")],
        );
        root.append_child(&container).unwrap();

        let layers_panel = el(
            document,
            "div",
            &style!("position: absolute; left: 16px; top: 16px; box-sizing: border-box; width: 200px; padding: 4px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "layers"), ("role", "listbox"), ("aria-label", "Layers")],
        );
        root.append_child(&layers_panel).unwrap();

        let inspector = el(
            document,
            "div",
            &style!("position: absolute; left: 728px; top: 16px; display: none; box-sizing: border-box; width: 216px; padding: 8px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "inspector"), ("role", "group"), ("aria-label", "Inspector")],
        );
        let title = el(
            document,
            "div",
            &style!("height: 24px; margin-bottom: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
            &[("id", "inspector-title")],
        );
        inspector.append_child(&title).unwrap();
        let mut values = Vec::new();
        let mut field_buttons = Vec::new();
        for (attr, label, step) in FIELDS {
            let field = el(
                document,
                "div",
                &style!("display: flex; column-gap: 4px; height: 24px; margin-bottom: 4px"),
                &[("id", &format!("field-{attr}"))],
            );
            let label_element = with_text(
                el(
                    document,
                    "div",
                    &style!("width: 56px; height: 24px; line-height: 24px; white-space: nowrap"),
                    &[],
                ),
                label,
            );
            field.append_child(&label_element).unwrap();
            let value = el(
                document,
                "div",
                &style!("width: 72px; height: 24px; line-height: 24px; white-space: nowrap; overflow: hidden"),
                &[("id", &format!("value-{attr}"))],
            );
            field.append_child(&value).unwrap();
            values.push(value);
            for (sign, text, name) in [(-1.0, "\u{2212}", "dec"), (1.0, "+", "inc")] {
                let verb = if name == "dec" {
                    "Decrease"
                } else {
                    "Increase"
                };
                let button = step_button(
                    document,
                    &[
                        ("id", &format!("{name}-{attr}")),
                        ("type", "button"),
                        ("title", &format!("{verb} {label}")),
                    ],
                    text,
                );
                field.append_child(&button).unwrap();
                field_buttons.push((button, attr, sign * step));
            }
            inspector.append_child(&field).unwrap();
        }
        let fill_row = el(
            document,
            "div",
            &style!("display: flex; column-gap: 4px; height: 24px; margin-bottom: 4px"),
            &[("id", "field-fill")],
        );
        let mut swatches = Vec::new();
        for fill in FILLS {
            let button = step_button(
                document,
                &[
                    ("id", &format!("fill-{}", &fill[1..])),
                    ("type", "button"),
                    ("title", fill),
                    ("aria-pressed", "false"),
                ],
                "",
            );
            button.set_background_color(hex_color(fill)).unwrap();
            fill_row.append_child(&button).unwrap();
            swatches.push((button, fill));
        }
        inspector.append_child(&fill_row).unwrap();
        let visible_button = with_text(
            el(
                document,
                "button",
                &style!("box-sizing: border-box; width: 72px; height: 24px; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
                &[("id", "toggle-visible"), ("type", "button"), ("aria-pressed", "true")],
            ),
            "Visible",
        );
        inspector.append_child(&visible_button).unwrap();
        root.append_child(&inspector).unwrap();

        // The scene.
        let stage = StageDom::new(document, &container, STAGE_WIDTH, STAGE_HEIGHT);
        let background = Node::new(
            ClassName::Layer,
            &[("id", "background".into()), ("name", "Background".into())],
        );
        background.add(&Node::new(
            ClassName::Rect,
            &[
                ("id", "sky".into()),
                ("name", "Sky".into()),
                ("x", 0.0.into()),
                ("y", 0.0.into()),
                ("width", 480.0.into()),
                ("height", 240.0.into()),
                ("fill", "#e7f5ff".into()),
            ],
        ));
        background.add(&Node::new(
            ClassName::Rect,
            &[
                ("id", "ground".into()),
                ("name", "Ground".into()),
                ("x", 0.0.into()),
                ("y", 240.0.into()),
                ("width", 480.0.into()),
                ("height", 120.0.into()),
                ("fill", "#ebfbee".into()),
            ],
        ));
        let shapes = Node::new(
            ClassName::Layer,
            &[("id", "shapes".into()), ("name", "Shapes".into())],
        );
        // div-gap: W2 W6. A translucent shape with a fill and a stroke is
        // drawn through Konva's isolation canvas, which the port can't draw
        // (it needs the context's getTransform and drawImage of a canvas),
        // so stroked shapes turn perfectDrawEnabled off, here and in the
        // reference page.
        shapes.add(&Node::new(
            ClassName::Rect,
            &[
                ("id", "red-box".into()),
                ("name", "Red box".into()),
                ("x", 60.0.into()),
                ("y", 60.0.into()),
                ("width", 120.0.into()),
                ("height", 80.0.into()),
                ("fill", "#e03131".into()),
                ("stroke", "#1e1e1e".into()),
                ("strokeWidth", 2.0.into()),
                ("cornerRadius", 8.0.into()),
                ("perfectDrawEnabled", false.into()),
            ],
        ));
        shapes.add(&Node::new(
            ClassName::Circle,
            &[
                ("id", "blue-dot".into()),
                ("name", "Blue dot".into()),
                ("x", 300.0.into()),
                ("y", 120.0.into()),
                ("radius", 40.0.into()),
                ("fill", "#1971c2".into()),
            ],
        ));
        let pair = Node::new(
            ClassName::Group,
            &[
                ("id", "pair".into()),
                ("name", "Pair".into()),
                ("x", 200.0.into()),
                ("y", 220.0.into()),
                ("rotation", 15.0.into()),
            ],
        );
        pair.add(&Node::new(
            ClassName::Rect,
            &[
                ("id", "pair-rect".into()),
                ("name", "Pair rect".into()),
                ("x", 0.0.into()),
                ("y", 0.0.into()),
                ("width", 60.0.into()),
                ("height", 40.0.into()),
                ("fill", "#2f9e44".into()),
                ("stroke", "#1e1e1e".into()),
                ("strokeWidth", 1.0.into()),
                ("dash", vec![4.0, 2.0].into()),
                ("perfectDrawEnabled", false.into()),
            ],
        ));
        pair.add(&Node::new(
            ClassName::Circle,
            &[
                ("id", "pair-dot".into()),
                ("name", "Pair dot".into()),
                ("x", 90.0.into()),
                ("y", 20.0.into()),
                ("radius", 20.0.into()),
                ("fill", "#f08c00".into()),
                ("opacity", 0.8.into()),
            ],
        ));
        shapes.add(&pair);
        stage.add(&background);
        stage.add(&shapes);

        let inner = Rc::new(Inner {
            document: document.clone(),
            root,
            stage,
            shapes,
            layers_panel,
            inspector,
            title,
            values,
            fill_row,
            swatches,
            visible_button,
            selected: RefCell::new(None),
            created: Cell::new(0),
        });

        for (button, id) in &tool_buttons {
            let action: fn(&Rc<Inner>) = match *id {
                "add-rect" => |i| add(i, ClassName::Rect),
                "add-circle" => |i| add(i, ClassName::Circle),
                "raise" => |i| restack(i, true),
                "lower" => |i| restack(i, false),
                _ => remove,
            };
            button.on("click", listener(&inner, action)).unwrap();
        }
        for (button, attr, delta) in field_buttons {
            button
                .on("click", listener(&inner, move |i| adjust(i, attr, delta)))
                .unwrap();
        }
        for (button, fill) in &inner.swatches {
            let fill = *fill;
            button
                .on("click", listener(&inner, move |i| set_fill(i, fill)))
                .unwrap();
        }
        inner
            .visible_button
            .on("click", listener(&inner, toggle_visible))
            .unwrap();
        let weak = Rc::downgrade(&inner);
        document
            .body()
            .on("keydown", move |event: &Event| {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                match event.key() {
                    Some(Key::Escape) => select(&inner, None),
                    Some(Key::Delete | Key::Backspace) => remove(&inner),
                    Some(Key::ArrowLeft) => nudge(&inner, -1.0, 0.0),
                    Some(Key::ArrowRight) => nudge(&inner, 1.0, 0.0),
                    Some(Key::ArrowUp) => nudge(&inner, 0.0, -1.0),
                    Some(Key::ArrowDown) => nudge(&inner, 0.0, 1.0),
                    _ => {}
                }
            })
            .unwrap();
        update(&inner);

        parent.append_child(&inner.root).unwrap();
        Shell(inner)
    }
}

fn selected_node(inner: &Inner) -> Option<Node> {
    let selected = inner.selected.borrow().clone()?;
    node_by_id(&inner.stage.stage.children(), &selected)
}

fn select(inner: &Rc<Inner>, id: Option<String>) {
    *inner.selected.borrow_mut() = id;
    update(inner);
}

fn adjust(inner: &Rc<Inner>, attr: &str, delta: f64) {
    let node = selected_node(inner).expect("a selected node");
    let mut value = number(&node, attr) + delta;
    if attr == "opacity" {
        value = js::min(&[1.0, js::max(&[0.0, value])]);
    }
    node.set_attr(attr, Value::Number(value));
    redraw(&node.layer().expect("a layer"));
    update(inner);
}

fn set_fill(inner: &Rc<Inner>, fill: &str) {
    let node = selected_node(inner).expect("a selected node");
    node.set_attr("fill", fill.into());
    redraw(&node.layer().expect("a layer"));
    update(inner);
}

fn toggle_visible(inner: &Rc<Inner>) {
    let node = selected_node(inner).expect("a selected node");
    node.set_attr("visible", Value::Bool(!node.visible()));
    redraw(&node.layer().expect("a layer"));
    update(inner);
}

fn nudge(inner: &Rc<Inner>, dx: f64, dy: f64) {
    let Some(node) = selected_node(inner) else {
        return;
    };
    node.set_attr("x", Value::Number(node.x() + dx));
    node.set_attr("y", Value::Number(node.y() + dy));
    redraw(&node.layer().expect("a layer"));
    update(inner);
}

fn add(inner: &Rc<Inner>, class: ClassName) {
    let target = match selected_node(inner) {
        None => inner.shapes.clone(),
        Some(node) if is_container(&node) => node,
        Some(node) => node.parent().expect("a parent"),
    };
    let n = inner.created.get() + 1;
    inner.created.set(n);
    let k = f64::from(n);
    let fill = FILLS[(n as usize - 1) % FILLS.len()];
    let shape = if class == ClassName::Rect {
        Node::new(
            ClassName::Rect,
            &[
                ("id", format!("rect-{n}").as_str().into()),
                ("name", format!("Rect {n}").as_str().into()),
                ("x", (20.0 + 30.0 * k).into()),
                ("y", (20.0 + 20.0 * k).into()),
                ("width", 80.0.into()),
                ("height", 50.0.into()),
                ("fill", fill.into()),
            ],
        )
    } else {
        Node::new(
            ClassName::Circle,
            &[
                ("id", format!("circle-{n}").as_str().into()),
                ("name", format!("Circle {n}").as_str().into()),
                ("x", (60.0 + 30.0 * k).into()),
                ("y", (60.0 + 20.0 * k).into()),
                ("radius", 30.0.into()),
                ("fill", fill.into()),
            ],
        )
    };
    target.add(&shape);
    *inner.selected.borrow_mut() = Some(string_attr(&shape, "id"));
    redraw(&target.layer().expect("a layer"));
    update(inner);
}

fn restack(inner: &Rc<Inner>, up: bool) {
    let Some(node) = selected_node(inner) else {
        return;
    };
    if node.node_type() == NodeType::Layer {
        return;
    }
    let moved = if up { node.move_up() } else { node.move_down() };
    if moved {
        redraw(&node.layer().expect("a layer"));
    }
    update(inner);
}

fn remove(inner: &Rc<Inner>) {
    let Some(node) = selected_node(inner) else {
        return;
    };
    if node.node_type() == NodeType::Layer {
        return;
    }
    let layer = node.layer().expect("a layer");
    node.destroy();
    *inner.selected.borrow_mut() = None;
    redraw(&layer);
    update(inner);
}

fn update(inner: &Rc<Inner>) {
    // The layers panel: topmost first at every level.
    let panel = &inner.layers_panel;
    while let Some(child) = panel.first_child().unwrap() {
        panel.remove_child(&child).unwrap();
    }
    let selected = inner.selected.borrow().clone();
    let mut rows = Vec::new();
    fn walk(
        inner: &Rc<Inner>,
        nodes: &[Node],
        depth: usize,
        selected: Option<&str>,
        rows: &mut Vec<Element>,
    ) {
        for node in nodes.iter().rev() {
            let id = string_attr(node, "id");
            let is_selected = selected == Some(id.as_str());
            let row = with_text(
                el(
                    &inner.document,
                    "div",
                    &row_style(depth),
                    &[
                        ("id", &format!("row-{id}")),
                        ("role", "option"),
                        ("aria-selected", if is_selected { "true" } else { "false" }),
                    ],
                ),
                &string_attr(node, "name"),
            );
            if is_selected {
                row.set_background_color(color(PRESSED)).unwrap();
            }
            row.on(
                "click",
                listener(inner, move |i| select(i, Some(id.clone()))),
            )
            .unwrap();
            rows.push(row);
            walk(inner, &children(node), depth + 1, selected, rows);
        }
    }
    walk(
        inner,
        &inner.stage.stage.children(),
        0,
        selected.as_deref(),
        &mut rows,
    );
    for row in &rows {
        panel.append_child(row).unwrap();
    }

    let node = selected_node(inner);
    inner
        .inspector
        .set_display(if node.is_some() {
            Display::Block
        } else {
            Display::None
        })
        .unwrap();
    let Some(node) = node else {
        return;
    };
    inner
        .title
        .set_text(&format!(
            "{} {}",
            node.class_name().as_str(),
            string_attr(&node, "id")
        ))
        .unwrap();
    for ((attr, _, _), value) in FIELDS.iter().zip(&inner.values) {
        value
            .set_text(&js::number_to_string(number(&node, attr)))
            .unwrap();
    }
    let is_shape = !is_container(&node);
    inner
        .fill_row
        .set_display(if is_shape {
            Display::Flex
        } else {
            Display::None
        })
        .unwrap();
    let fill = string_attr(&node, "fill");
    for (button, swatch) in &inner.swatches {
        let pressed = is_shape && node.attr("fill").is_some() && fill == *swatch;
        button
            .set_attribute("aria-pressed", if pressed { "true" } else { "false" })
            .unwrap();
        set_border_color(
            button,
            if pressed {
                SELECTED_SWATCH
            } else {
                SWATCH_BORDER
            },
        );
    }
    let visible = node.visible();
    inner
        .visible_button
        .set_attribute("aria-pressed", if visible { "true" } else { "false" })
        .unwrap();
    inner
        .visible_button
        .set_background_color(color(if visible { PRESSED } else { WHITE }))
        .unwrap();
}
