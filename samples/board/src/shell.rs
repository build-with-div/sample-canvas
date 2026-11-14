//! The board's shell on Div: a `<canvas>` under a toolbar, a style panel and
//! a layers list, with the pointer gestures and shortcuts of an
//! Excalidraw-style board.
//!
//! The DOM and its inline styles are those of the reference page,
//! `tools/board-reference/board.js`, element for element, so that
//! `tests/fidelity_board.rs` can compare the two in pinned Chromium.

use crate::scene::{Scene, StyleChange, Tool, STYLE_SECTIONS};
use div::prelude::*;
use div::typed_style::{Color, Display, Style, StyleColor};
use roughjs::{CallRecorder, CanvasCall, RoughCanvas};
use std::cell::RefCell;
use std::rc::Rc;

/// The board's size in CSS pixels.
pub const WIDTH: f32 = 960.0;
pub const HEIGHT: f32 = 600.0;

const PRESSED: u32 = 0xe0dfff;
const SELECTED_SWATCH: u32 = 0x4a47b1;
const SWATCH_BORDER: u32 = 0xcccccc;
const WHITE: u32 = 0xffffff;

/// A mounted board.
#[derive(Clone)]
pub struct Board {
    pub root: Element,
    pub scene: Signal<Scene>,
    /// The 2D context calls of the last redraw.
    pub trace: Rc<RefCell<Vec<CanvasCall>>>,
}

fn color(hex: u32) -> Color {
    Color::from_hex(hex, false)
}

/// `#rrggbb`, or `transparent`.
fn css_color(value: &str) -> Color {
    match value.strip_prefix('#') {
        Some(hex) => color(u32::from_str_radix(hex, 16).expect("a hex color")),
        None => Color::TRANSPARENT,
    }
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
// unitless integer (sample-git gap 4). So the board's styles spell out the
// twelve longhands, with `1` for `1px`, where the reference page uses the
// three shorthands. The computed style is the same.
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

/// The key as `KeyboardEvent.key` names it.
fn key_name(event: &Event) -> Option<String> {
    Some(match event.key()? {
        Key::Char(c) => c.to_string(),
        Key::Escape => "Escape".into(),
        Key::Delete => "Delete".into(),
        Key::Backspace => "Backspace".into(),
        Key::Enter => "Enter".into(),
        Key::Tab => "Tab".into(),
        Key::Space => " ".into(),
        _ => return None,
    })
}

impl Board {
    /// Builds the board and appends it to `parent`.
    pub fn mount(document: &Document, parent: &Element) -> Board {
        with_document(document, || Self::build(document, parent))
    }

    fn build(document: &Document, parent: &Element) -> Board {
        let scene = create_signal(Scene::default());
        let trace = Rc::new(RefCell::new(Vec::new()));

        let root = el(
            document,
            "div",
            &style!("position: relative; width: 960px; height: 600px; overflow: hidden; background-color: #ffffff; font-family: sans-serif; font-size: 12px; line-height: 16px; color: #1e1e1e"),
            &[("id", "board")],
        );
        // div-gap: W1 HTMLCanvasElement. Div's <canvas> lays out but has no
        // bitmap (no `width`/`height` attributes, no getContext); the
        // board sizes it with CSS only.
        let canvas = el(
            document,
            "canvas",
            &style!("position: absolute; left: 0px; top: 0px; display: block; width: 960px; height: 600px"),
            &[("id", "scene")],
        );
        root.append_child(&canvas).unwrap();

        // The toolbar.
        let toolbar = el(
            document,
            "div",
            &style!("position: absolute; left: 377px; top: 16px; display: flex; column-gap: 4px; padding: 4px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "toolbar"), ("role", "toolbar"), ("aria-label", "Tools")],
        );
        let mut tool_buttons = Vec::new();
        for tool in Tool::ALL {
            let button = with_text(
                el(
                    document,
                    "button",
                    &style!("box-sizing: border-box; width: 36px; height: 36px; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #ffffff; border-right-color: #ffffff; border-bottom-color: #ffffff; border-left-color: #ffffff; border-radius: 6px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
                    &[
                        ("id", &format!("tool-{}", tool.name())),
                        ("type", "button"),
                        ("data-tool", tool.name()),
                        ("title", tool.title()),
                        ("aria-pressed", "false"),
                    ],
                ),
                tool.label(),
            );
            button
                .on("click", move |_: &Event| scene.update(|s| s.set_tool(tool)))
                .unwrap();
            toolbar.append_child(&button).unwrap();
            tool_buttons.push((button, tool));
        }
        root.append_child(&toolbar).unwrap();

        // The style panel.
        let style_panel = el(
            document,
            "div",
            &style!("position: absolute; left: 16px; top: 16px; display: none; box-sizing: border-box; width: 184px; padding: 12px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "style-panel"), ("role", "group"), ("aria-label", "Style")],
        );
        let mut option_buttons = Vec::new();
        for (index, section) in STYLE_SECTIONS.iter().enumerate() {
            let section_style = if index == 0 {
                style!("margin-top: 0px")
            } else {
                style!("margin-top: 12px")
            };
            let section_element = el(
                document,
                "div",
                &section_style,
                &[("id", &format!("section-{}", section.name))],
            );
            let label = with_text(
                el(
                    document,
                    "div",
                    &style!("height: 16px; margin-bottom: 6px; white-space: nowrap"),
                    &[("id", &format!("label-{}", section.name))],
                ),
                section.label,
            );
            section_element.append_child(&label).unwrap();
            let row = el(
                document,
                "div",
                &style!("display: flex; column-gap: 6px; height: 28px"),
                &[("id", &format!("options-{}", section.name))],
            );
            let is_color = matches!(section.name, "stroke" | "background");
            for (value, text) in section.options {
                let button = with_text(
                    el(
                        document,
                        "button",
                        &style!("box-sizing: border-box; width: 28px; height: 28px; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
                        &[
                            ("id", &format!("{}-{}", section.name, value.replace('#', ""))),
                            ("type", "button"),
                            ("data-section", section.name),
                            ("data-value", value),
                            ("title", value),
                            ("aria-pressed", "false"),
                        ],
                    ),
                    text,
                );
                if is_color {
                    button.set_background_color(css_color(value)).unwrap();
                }
                let change = StyleChange::from_option(section.name, value);
                button
                    .on("click", move |_: &Event| {
                        scene.update(|s| s.set_style(&change))
                    })
                    .unwrap();
                row.append_child(&button).unwrap();
                option_buttons.push((button, section.name, *value, is_color));
            }
            section_element.append_child(&row).unwrap();
            style_panel.append_child(&section_element).unwrap();
        }
        root.append_child(&style_panel).unwrap();

        // The layers list, topmost element first.
        let layers = el(
            document,
            "div",
            &style!("position: absolute; left: 768px; top: 16px; display: none; box-sizing: border-box; width: 176px; padding: 4px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "layers"), ("role", "listbox"), ("aria-label", "Elements")],
        );
        let row_document = document.clone();
        let rows = For(
            move || {
                scene
                    .get()
                    .elements
                    .iter()
                    .rev()
                    .map(|e| e.id.clone())
                    .collect::<Vec<_>>()
            },
            |id| id.clone(),
            move |id: String| {
                let row = el(
                    &row_document,
                    "div",
                    &style!("height: 28px; padding-left: 8px; padding-right: 8px; border-radius: 4px; line-height: 28px; white-space: nowrap; overflow: hidden"),
                    &[
                        ("id", &format!("layer-{id}")),
                        ("role", "option"),
                        ("aria-selected", "false"),
                    ],
                );
                let weak = row.downgrade();
                let row_id = id.clone();
                create_effect(move || {
                    let s = scene.get();
                    let (Some(row), Some(element)) = (weak.upgrade(), s.element(&row_id)) else {
                        return;
                    };
                    let selected = s.selected.as_deref() == Some(row_id.as_str());
                    row.set_text(&element.name).unwrap();
                    row.set_attribute("aria-selected", if selected { "true" } else { "false" })
                        .unwrap();
                    row.set_background_color(if selected {
                        color(PRESSED)
                    } else {
                        Color::TRANSPARENT
                    })
                    .unwrap();
                });
                row.on("click", move |_: &Event| scene.update(|s| s.select(&id)))
                    .unwrap();
                ViewNode::Element(row)
            },
        );
        mount_view(&layers, rows);
        root.append_child(&layers).unwrap();

        // Pointer gestures. Coordinates are scene coordinates, as
        // Excalidraw computes them: clientX/clientY minus the canvas's
        // getBoundingClientRect().
        // div-gap: W14 Pointer Events. Div fires mouse* for the mouse and
        // pointer* only for touch, so the board listens for
        // mousedown/mousemove/mouseup where the reference page listens for
        // pointerdown/pointermove/pointerup.
        let scene_point = {
            let canvas = canvas.clone();
            move |event: &Event| {
                let rect = canvas
                    .bounding_rect()
                    .unwrap()
                    .expect("the canvas has a box");
                (
                    f64::from(event.mouse_x) - f64::from(rect.x),
                    f64::from(event.mouse_y) - f64::from(rect.y),
                )
            }
        };
        let down_point = scene_point.clone();
        canvas
            .on("mousedown", move |event: &Event| {
                if event.mouse_button != 0 {
                    return;
                }
                let (x, y) = down_point(event);
                scene.update(|s| s.pointer_down(x, y));
            })
            .unwrap();
        let body = document.body();
        body.on("mousemove", move |event: &Event| {
            if scene.get().drag.is_some() {
                let (x, y) = scene_point(event);
                scene.update(|s| {
                    s.pointer_move(x, y);
                });
            }
        })
        .unwrap();
        body.on("mouseup", move |event: &Event| {
            if event.mouse_button == 0 && scene.get().drag.is_some() {
                scene.update(|s| {
                    s.pointer_up();
                });
            }
        })
        .unwrap();
        body.on("keydown", move |event: &Event| {
            let Some(key) = key_name(event) else {
                return;
            };
            let mut next = scene.get();
            if next.key(&key) {
                scene.set(next);
            }
        })
        .unwrap();

        // Rendering: the DOM state, then the scene.
        let rc = Rc::new(RefCell::new(RoughCanvas::new(
            CallRecorder::default(),
            None,
        )));
        let redraw_trace = trace.clone();
        create_effect(move || {
            let s = scene.get();
            for (button, tool) in &tool_buttons {
                let pressed = s.tool == *tool;
                button
                    .set_attribute("aria-pressed", if pressed { "true" } else { "false" })
                    .unwrap();
                button
                    .set_background_color(color(if pressed { PRESSED } else { WHITE }))
                    .unwrap();
            }
            for (button, section, value, is_color) in &option_buttons {
                let pressed = s.style.value(section) == *value;
                button
                    .set_attribute("aria-pressed", if pressed { "true" } else { "false" })
                    .unwrap();
                if *is_color {
                    set_border_color(
                        button,
                        if pressed {
                            SELECTED_SWATCH
                        } else {
                            SWATCH_BORDER
                        },
                    );
                } else {
                    button
                        .set_background_color(color(if pressed { PRESSED } else { WHITE }))
                        .unwrap();
                }
            }
            let display = |shown: bool| if shown { Display::Block } else { Display::None };
            style_panel
                .set_display(display(s.shows_style_panel()))
                .unwrap();
            layers.set_display(display(!s.elements.is_empty())).unwrap();

            // div-gap: W11 requestAnimationFrame. Div has no frame
            // callback, so the scene is redrawn as soon as it changes,
            // where the reference page redraws in requestAnimationFrame.
            // div-gap: W2 CanvasRenderingContext2D. The redraw goes to a
            // CallRecorder, which draws nothing.
            let mut rc = rc.borrow_mut();
            rc.context_mut().calls.clear();
            s.draw(&mut rc);
            *redraw_trace.borrow_mut() = rc.context().calls.clone();
        });

        parent.append_child(&root).unwrap();
        Board { root, scene, trace }
    }
}
