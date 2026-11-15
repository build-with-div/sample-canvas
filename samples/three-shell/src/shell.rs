//! The three.js example shell on Div: a toolbar, an objects pane, a
//! splitter and a viewport whose `<canvas>` OrbitControls drives, with a
//! settings popover over the canvas.
//!
//! The DOM, its inline styles and the app code are those of the reference
//! page, `tools/three-reference/shell.js`, element for element and step for
//! step, so that `tests/fidelity_shell.rs` can compare the two in pinned
//! Chromium.

use div::prelude::*;
use div::typed_style::{Color, Display, Length, LengthValue};
use std::cell::{Cell, Ref, RefCell};
use std::rc::{Rc, Weak};
use three::{
    ControlsEvent, Element as ControlsElement, Modifiers as ControlsModifiers, OrbitControls,
    PerspectiveCamera, PointerEvent, Vector3, WheelEvent,
};

/// The shell's size in CSS pixels.
pub const WIDTH: f32 = 960.0;
pub const HEIGHT: f32 = 600.0;

const PRESSED: u32 = 0xe0dfff;
const WHITE: u32 = 0xffffff;
const SPLITTER: f64 = 6.0;
const MIN_PANE: f64 = 140.0;
const MAX_PANE: f64 = 360.0;
/// (id, label, position)
const OBJECTS: [(&str, &str, [f64; 3]); 3] = [
    ("origin", "Origin", [0.0, 0.0, 0.0]),
    ("box", "Box", [2.0, 0.5, -1.0]),
    ("sphere", "Sphere", [-1.5, 1.0, 1.5]),
];
/// (id, label, rotateSpeed)
const SPEEDS: [(&str, &str, f64); 3] = [
    ("speed-half", "0.5x", 0.5),
    ("speed-1", "1x", 1.0),
    ("speed-2", "2x", 2.0),
];

/// A mounted shell.
#[derive(Clone)]
pub struct Shell(Rc<Inner>);

struct Inner {
    root: Element,
    auto_rotate: Element,
    damping: Element,
    settings_button: Element,
    status: Element,
    objects: Element,
    rows: Vec<(Element, &'static str)>,
    splitter: Element,
    viewport: Element,
    canvas: Element,
    settings: Element,
    speed_buttons: Vec<(Element, f64)>,
    screen_space: Element,
    controls: RefCell<OrbitControls>,
    /// The controls' events since the last snapshot, as the page's
    /// listeners record them.
    events: RefCell<Vec<&'static str>>,
    selected: Cell<&'static str>,
    settings_open: Cell<bool>,
    pane_width: Cell<f64>,
    /// The splitter drag: the pointer's x and the pane width at its start.
    drag: Cell<Option<(f64, f64)>>,
    /// The viewport size the last resize saw.
    viewport_size: Cell<Option<(f64, f64)>>,
}

fn color(hex: u32) -> Color {
    Color::from_hex(hex, false)
}

// div-gap: 15 CSS border shorthands. Div's `style!` rejects `border-width`,
// `border-style` and `border-color`, and takes a border width only as a
// unitless integer. So the shell's styles spell out the longhands, with `1`
// for `1px`, where the reference page uses the shorthands. The computed
// style is the same.
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

fn tool_button(document: &Document, attributes: &[(&str, &str)], text: &str) -> Element {
    with_text(
        el(
            document,
            "button",
            &style!("width: 88px; height: 28px; box-sizing: border-box; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
            attributes,
        ),
        text,
    )
}

fn speed_button(document: &Document, attributes: &[(&str, &str)], text: &str) -> Element {
    with_text(
        el(
            document,
            "button",
            &style!("width: 32px; height: 24px; box-sizing: border-box; margin: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
            attributes,
        ),
        text,
    )
}

fn wide_button(document: &Document, attributes: &[(&str, &str)], text: &str) -> Element {
    with_text(
        el(
            document,
            "button",
            &style!("display: block; width: 166px; height: 24px; margin-bottom: 4px; box-sizing: border-box; margin-top: 0px; margin-left: 0px; margin-right: 0px; padding: 0px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #cccccc; border-right-color: #cccccc; border-bottom-color: #cccccc; border-left-color: #cccccc; border-radius: 4px; background-color: #ffffff; color: #1e1e1e; font-family: sans-serif; font-size: 12px; line-height: 16px"),
            attributes,
        ),
        text,
    )
}

fn px(value: f64) -> LengthValue {
    LengthValue::px(value as f32)
}

/// `aria-pressed` and the pressed background.
fn set_pressed(button: &Element, pressed: bool) {
    button
        .set_attribute("aria-pressed", if pressed { "true" } else { "false" })
        .unwrap();
    button
        .set_background_color(color(if pressed { PRESSED } else { WHITE }))
        .unwrap();
}

/// What JavaScript's `String(n)` gives for the integers the pane width
/// takes.
fn integer_string(value: f64) -> String {
    debug_assert_eq!(value.fract(), 0.0);
    format!("{}", value as i64)
}

/// `event.ctrlKey`, `metaKey` and `shiftKey`.
fn controls_modifiers(event: &Event) -> ControlsModifiers {
    let bits = event.modifiers as u32;
    ControlsModifiers {
        ctrl: bits & Modifiers::CTRL.bits() != 0,
        meta: bits & Modifiers::META.bits() != 0,
        shift: bits & Modifiers::SHIFT.bits() != 0,
    }
}

/// The controls' view of a mouse event.
///
/// div-gap: W14 Pointer Events. Div fires `mouse*` for the mouse, so the
/// shell feeds OrbitControls `mousedown`/`mousemove`/`mouseup` where the
/// page's OrbitControls listens for `pointerdown`/`pointermove`/
/// `pointerup`. The mouse's `pointerId` is 1, as in Chromium (decision 4).
fn pointer(event: &Event) -> PointerEvent {
    PointerEvent {
        pointer_id: 1,
        button: event.mouse_button,
        client_x: f64::from(event.mouse_x),
        client_y: f64::from(event.mouse_y),
        modifiers: controls_modifiers(event),
    }
}

/// `event.code` for the keys OrbitControls handles.
///
/// div-gap: 20 `KeyboardEvent.code`. Div's key events carry the `key` only;
/// for the arrow keys the `code` has the same name.
fn key_code(event: &Event) -> Option<&'static str> {
    match event.key()? {
        Key::ArrowUp => Some("ArrowUp"),
        Key::ArrowDown => Some("ArrowDown"),
        Key::ArrowLeft => Some("ArrowLeft"),
        Key::ArrowRight => Some("ArrowRight"),
        _ => None,
    }
}

/// Runs `f` on the shell from an event listener, unless it is gone.
fn listener(
    inner: &Rc<Inner>,
    f: impl Fn(&Rc<Inner>, &Event) + 'static,
) -> impl Fn(&Event) + 'static {
    let weak: Weak<Inner> = Rc::downgrade(inner);
    move |event: &Event| {
        if let Some(inner) = weak.upgrade() {
            f(&inner, event);
        }
    }
}

impl Shell {
    /// Builds the shell and appends it to `parent`.
    pub fn mount(document: &Document, parent: &Element) -> Shell {
        let shell = with_document(document, || Self::build(document, parent));
        // The ResizeObserver's first notification.
        shell.0.resize();
        shell
    }

    /// The shell's root element, `#app`.
    pub fn root(&self) -> Element {
        self.0.root.clone()
    }

    /// OrbitControls and its camera.
    pub fn controls(&self) -> Ref<'_, OrbitControls> {
        self.0.controls.borrow()
    }

    /// The selected object's id.
    pub fn selected(&self) -> &'static str {
        self.0.selected.get()
    }

    /// Takes the controls' events since the last call.
    pub fn take_events(&self) -> Vec<&'static str> {
        std::mem::take(&mut *self.0.events.borrow_mut())
    }

    /// One animation frame: `controls.update()`.
    ///
    /// div-gap: W11 requestAnimationFrame. The page calls this from the
    /// capture after each step, once, where an app would update the
    /// controls in its animation loop; Div has no frame callback, and a
    /// fixed frame per step keeps damping and auto-rotation deterministic
    /// on both sides.
    pub fn frame(&self) {
        self.0.with_controls(|controls| {
            controls.update(None);
        });
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
            &style!("position: absolute; left: 0px; top: 0px; box-sizing: border-box; width: 960px; height: 40px; display: flex; column-gap: 4px; padding-top: 6px; padding-left: 8px; border-bottom-width: 1; border-bottom-style: solid; border-bottom-color: #dddddd; background-color: #ffffff"),
            &[("id", "toolbar"), ("role", "toolbar"), ("aria-label", "View")],
        );
        let reset = tool_button(document, &[("id", "reset"), ("type", "button")], "Reset");
        let auto_rotate = tool_button(
            document,
            &[
                ("id", "auto-rotate"),
                ("type", "button"),
                ("aria-pressed", "false"),
            ],
            "Auto-rotate",
        );
        let damping = tool_button(
            document,
            &[
                ("id", "damping"),
                ("type", "button"),
                ("aria-pressed", "false"),
            ],
            "Damping",
        );
        let settings_button = tool_button(
            document,
            &[
                ("id", "settings-button"),
                ("type", "button"),
                ("aria-expanded", "false"),
            ],
            "Settings",
        );
        let status = with_text(
            el(
                document,
                "div",
                &style!("width: 120px; height: 28px; padding-left: 8px; line-height: 28px; white-space: nowrap; overflow: hidden"),
                &[("id", "status"), ("role", "status")],
            ),
            "Idle",
        );
        for element in [&reset, &auto_rotate, &damping, &settings_button, &status] {
            toolbar.append_child(element).unwrap();
        }
        root.append_child(&toolbar).unwrap();

        let objects = el(
            document,
            "div",
            &style!("position: absolute; left: 0px; top: 40px; box-sizing: border-box; width: 200px; height: 560px; padding: 8px; background-color: #ffffff"),
            &[("id", "objects"), ("role", "listbox"), ("aria-label", "Objects")],
        );
        let mut rows = Vec::new();
        for (id, label, _) in OBJECTS {
            let row = with_text(
                el(
                    document,
                    "div",
                    &style!("box-sizing: border-box; height: 24px; padding-left: 8px; border-radius: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
                    &[
                        ("id", &format!("object-{id}")),
                        ("role", "option"),
                        ("aria-selected", "false"),
                    ],
                ),
                label,
            );
            objects.append_child(&row).unwrap();
            rows.push((row, id));
        }
        root.append_child(&objects).unwrap();

        let splitter = el(
            document,
            "div",
            &style!("position: absolute; left: 200px; top: 40px; width: 6px; height: 560px; background-color: #dddddd"),
            &[
                ("id", "splitter"),
                ("role", "separator"),
                ("aria-orientation", "vertical"),
                ("aria-label", "Resize panes"),
                ("aria-valuenow", "200"),
            ],
        );
        root.append_child(&splitter).unwrap();

        let viewport = el(
            document,
            "div",
            &style!("position: absolute; left: 206px; right: 0px; top: 40px; bottom: 0px; overflow: hidden; background-color: #20232a"),
            &[("id", "viewport")],
        );
        let canvas = el(
            document,
            "canvas",
            &style!("display: block"),
            &[("id", "view")],
        );
        viewport.append_child(&canvas).unwrap();

        let settings = el(
            document,
            "div",
            &style!("position: absolute; top: 8px; right: 8px; display: none; box-sizing: border-box; width: 184px; padding: 8px; border-top-width: 1; border-right-width: 1; border-bottom-width: 1; border-left-width: 1; border-top-style: solid; border-right-style: solid; border-bottom-style: solid; border-left-style: solid; border-top-color: #dddddd; border-right-color: #dddddd; border-bottom-color: #dddddd; border-left-color: #dddddd; border-radius: 8px; background-color: #ffffff"),
            &[("id", "settings"), ("role", "dialog"), ("aria-label", "Settings")],
        );
        let title = with_text(
            el(
                document,
                "div",
                &style!("height: 24px; margin-bottom: 4px; line-height: 24px; white-space: nowrap; overflow: hidden"),
                &[("id", "settings-title")],
            ),
            "Settings",
        );
        settings.append_child(&title).unwrap();
        let speed_row = el(
            document,
            "div",
            &style!("display: flex; column-gap: 4px; height: 24px; margin-bottom: 4px"),
            &[("id", "speed-row")],
        );
        let speed_label = with_text(
            el(
                document,
                "div",
                &style!("width: 56px; height: 24px; line-height: 24px; white-space: nowrap"),
                &[],
            ),
            "Rotate",
        );
        speed_row.append_child(&speed_label).unwrap();
        let mut speed_buttons = Vec::new();
        for (id, label, speed) in SPEEDS {
            let button = speed_button(
                document,
                &[("id", id), ("type", "button"), ("aria-pressed", "false")],
                label,
            );
            speed_row.append_child(&button).unwrap();
            speed_buttons.push((button, speed));
        }
        settings.append_child(&speed_row).unwrap();
        let screen_space = wide_button(
            document,
            &[
                ("id", "screen-space"),
                ("type", "button"),
                ("aria-pressed", "false"),
            ],
            "Screen-space pan",
        );
        settings.append_child(&screen_space).unwrap();
        let close = wide_button(
            document,
            &[("id", "close-settings"), ("type", "button")],
            "Close",
        );
        settings.append_child(&close).unwrap();
        viewport.append_child(&settings).unwrap();
        root.append_child(&viewport).unwrap();
        parent.append_child(&root).unwrap();

        // The camera and its controls, as the page makes them.
        let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
        camera.position = Vector3::new(4.0, 3.0, 6.0);
        // div-gap: 18 `touch-action`. OrbitControls sets the canvas's
        // `style.touchAction = 'none'`; Div has no `touch-action` property
        // (and no touch input to scroll), so the canvas keeps its style.
        let controls = OrbitControls::new(
            camera,
            ControlsElement {
                client_width: 0.0,
                client_height: 0.0,
            },
        );

        let inner = Rc::new(Inner {
            root,
            auto_rotate,
            damping,
            settings_button,
            status,
            objects,
            rows,
            splitter,
            viewport,
            canvas,
            settings,
            speed_buttons,
            screen_space,
            controls: RefCell::new(controls),
            events: RefCell::new(Vec::new()),
            selected: Cell::new("origin"),
            settings_open: Cell::new(false),
            pane_width: Cell::new(200.0),
            drag: Cell::new(None),
            viewport_size: Cell::new(None),
        });

        // The controls' initial state, shown.
        {
            let controls = inner.controls.borrow();
            set_pressed(&inner.auto_rotate, controls.auto_rotate);
            set_pressed(&inner.damping, controls.enable_damping);
            set_pressed(&inner.screen_space, controls.screen_space_panning);
            for (button, speed) in &inner.speed_buttons {
                set_pressed(button, *speed == controls.rotate_speed);
            }
        }
        inner.show_selection();

        // The toolbar and the popover.
        reset
            .on(
                "click",
                listener(&inner, |inner, _| {
                    inner.with_controls(|controls| controls.reset());
                }),
            )
            .unwrap();
        inner
            .auto_rotate
            .on(
                "click",
                listener(&inner, |inner, _| {
                    let on = inner.with_controls(|controls| {
                        controls.auto_rotate = !controls.auto_rotate;
                        controls.auto_rotate
                    });
                    set_pressed(&inner.auto_rotate, on);
                }),
            )
            .unwrap();
        inner
            .damping
            .on(
                "click",
                listener(&inner, |inner, _| {
                    let on = inner.with_controls(|controls| {
                        controls.enable_damping = !controls.enable_damping;
                        controls.enable_damping
                    });
                    set_pressed(&inner.damping, on);
                }),
            )
            .unwrap();
        inner
            .settings_button
            .on(
                "click",
                listener(&inner, |inner, _| {
                    inner.open_settings(!inner.settings_open.get());
                }),
            )
            .unwrap();
        for (button, speed) in &inner.speed_buttons {
            let speed = *speed;
            button
                .on(
                    "click",
                    listener(&inner, move |inner, _| {
                        inner.with_controls(|controls| controls.rotate_speed = speed);
                        for (button, s) in &inner.speed_buttons {
                            set_pressed(button, *s == speed);
                        }
                    }),
                )
                .unwrap();
        }
        inner
            .screen_space
            .on(
                "click",
                listener(&inner, |inner, _| {
                    let on = inner.with_controls(|controls| {
                        controls.screen_space_panning = !controls.screen_space_panning;
                        controls.screen_space_panning
                    });
                    set_pressed(&inner.screen_space, on);
                }),
            )
            .unwrap();
        close
            .on(
                "click",
                listener(&inner, |inner, _| inner.open_settings(false)),
            )
            .unwrap();

        // The objects: selecting one moves the controls' target to it.
        for (row, id) in &inner.rows {
            let id = *id;
            row.on(
                "click",
                listener(&inner, move |inner, _| {
                    inner.selected.set(id);
                    inner.show_selection();
                    let [x, y, z] = OBJECTS.iter().find(|o| o.0 == id).unwrap().2;
                    inner.with_controls(|controls| {
                        controls.target = Vector3::new(x, y, z);
                        controls.update(None);
                    });
                }),
            )
            .unwrap();
        }

        // The splitter, and OrbitControls on the canvas.
        // div-gap: W14 Pointer Events: `mouse*` where the page listens for
        // `pointer*` (see `pointer`).
        inner
            .splitter
            .on(
                "mousedown",
                listener(&inner, |inner, event| {
                    if event.mouse_button == 0 {
                        inner
                            .drag
                            .set(Some((f64::from(event.mouse_x), inner.pane_width.get())));
                    }
                }),
            )
            .unwrap();
        inner
            .canvas
            .on(
                "mousedown",
                listener(&inner, |inner, event| {
                    let pointer = pointer(event);
                    inner.with_controls(|controls| controls.pointer_down(&pointer));
                }),
            )
            .unwrap();
        // div-gap: 19 `WheelEvent.deltaMode`. Div's wheel events carry
        // pixel deltas (as `f32`) and no `deltaMode`, so the controls see
        // `deltaMode` 0, which is what Chromium reports for these events.
        inner
            .canvas
            .on(
                "wheel",
                listener(&inner, |inner, event| {
                    event.prevent_default();
                    let wheel = WheelEvent {
                        client_x: f64::from(event.mouse_x),
                        client_y: f64::from(event.mouse_y),
                        delta_y: f64::from(event.delta_y),
                        delta_mode: 0,
                        ctrl: controls_modifiers(event).ctrl,
                    };
                    inner.with_controls(|controls| controls.wheel(&wheel));
                }),
            )
            .unwrap();
        // div-gap: 12 `contextmenu`. OrbitControls cancels the canvas's
        // context menu; Div has none to cancel.
        let body = document.body();
        body.on(
            "mousemove",
            listener(&inner, |inner, event| {
                if let Some((x, width)) = inner.drag.get() {
                    let next = (width + f64::from(event.mouse_x) - x).clamp(MIN_PANE, MAX_PANE);
                    inner.set_pane(next);
                } else {
                    let pointer = pointer(event);
                    inner.with_controls(|controls| controls.pointer_move(&pointer));
                }
            }),
        )
        .unwrap();
        body.on(
            "mouseup",
            listener(&inner, |inner, event| {
                if inner.drag.get().is_some() {
                    if event.mouse_button == 0 {
                        inner.drag.set(None);
                    }
                } else {
                    let pointer = pointer(event);
                    inner.with_controls(|controls| controls.pointer_up(&pointer));
                }
            }),
        )
        .unwrap();
        // `controls.listenToKeyEvents(window)`, and Escape closes the
        // popover.
        // div-gap: 20 a modifier key alone. OrbitControls also tracks
        // Control on the document (for ctrl-wheel zoom); Div reports no key
        // for Control, so that tracking isn't wired.
        body.on(
            "keydown",
            listener(&inner, |inner, event| {
                if event.key() == Some(Key::Escape) && inner.settings_open.get() {
                    inner.open_settings(false);
                }
                if let Some(code) = key_code(event) {
                    let modifiers = controls_modifiers(event);
                    inner.with_controls(|controls| controls.key_down(code, modifiers));
                }
            }),
        )
        .unwrap();

        Shell(inner)
    }
}

impl Inner {
    /// Runs `f` on the controls, then dispatches the events it fired to the
    /// page's listeners: the event log and the status line.
    fn with_controls<T>(&self, f: impl FnOnce(&mut OrbitControls) -> T) -> T {
        let (result, fired) = {
            let mut controls = self.controls.borrow_mut();
            let result = f(&mut controls);
            (result, std::mem::take(&mut controls.events))
        };
        for event in fired {
            self.events.borrow_mut().push(event.as_str());
            match event {
                ControlsEvent::Start => self.status.set_text("Moving").unwrap(),
                ControlsEvent::End => self.status.set_text("Idle").unwrap(),
                ControlsEvent::Change => {}
            }
        }
        result
    }

    fn show_selection(&self) {
        let selected = self.selected.get();
        for (row, id) in &self.rows {
            let on = *id == selected;
            row.set_attribute("aria-selected", if on { "true" } else { "false" })
                .unwrap();
            row.set_background_color(color(if on { PRESSED } else { WHITE }))
                .unwrap();
        }
    }

    fn open_settings(&self, open: bool) {
        self.settings_open.set(open);
        self.settings
            .set_display(if open { Display::Block } else { Display::None })
            .unwrap();
        self.settings_button
            .set_attribute("aria-expanded", if open { "true" } else { "false" })
            .unwrap();
        self.settings_button
            .set_background_color(color(if open { PRESSED } else { WHITE }))
            .unwrap();
    }

    fn set_pane(&self, width: f64) {
        self.pane_width.set(width);
        self.objects.set_width(px(width)).unwrap();
        self.splitter.set_left(Length::px(width as f32)).unwrap();
        self.viewport
            .set_left(Length::px((width + SPLITTER) as f32))
            .unwrap();
        self.splitter
            .set_attribute("aria-valuenow", &integer_string(width))
            .unwrap();
        self.resize();
    }

    /// The page's ResizeObserver on the viewport: the camera's aspect and
    /// the canvas's size follow the viewport's.
    ///
    /// div-gap: W13 ResizeObserver. Div has none, so the shell calls this
    /// after mounting and after each change that can resize the viewport,
    /// and it acts only when the size changed, as a notification would.
    fn resize(&self) {
        let rect = self
            .viewport
            .bounding_rect()
            .unwrap()
            .expect("the viewport has a box");
        let size = (f64::from(rect.width), f64::from(rect.height));
        if self.viewport_size.get() == Some(size) {
            return;
        }
        self.viewport_size.set(Some(size));
        let (width, height) = size;
        self.with_controls(|controls| {
            controls.camera.aspect = width / height;
            controls.camera.update_projection_matrix();
            // The canvas's clientWidth and clientHeight, after setSize.
            controls.element = ControlsElement {
                client_width: width,
                client_height: height,
            };
        });
        self.set_size(width, height);
    }

    /// What `renderer.setSize(width, height)` does to the canvas, at pixel
    /// ratio 1: the bitmap size as the `width` and `height` attributes, and
    /// the CSS size.
    ///
    /// div-gap: W17 WebGPU canvas. There is no renderer on either side yet;
    /// the page and the shell size the canvas as three.js's renderer would.
    fn set_size(&self, width: f64, height: f64) {
        self.canvas
            .set_attribute("width", &integer_string(width.floor()))
            .unwrap();
        self.canvas
            .set_attribute("height", &integer_string(height.floor()))
            .unwrap();
        self.canvas.set_width(px(width)).unwrap();
        self.canvas.set_height(px(height)).unwrap();
    }
}
