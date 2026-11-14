//! The board's scene: its elements, the selection, the active tool, the
//! style for new elements, and what a pointer gesture or a key does to
//! them.
//!
//! This is the board's own model, written in the style of Excalidraw's
//! (elements with a type, a box, stroke and background colors, a stroke
//! width, a roughness, a fill style and a seed). The reference page,
//! `tools/board-reference/board.js`, implements the same model in
//! JavaScript, and `tests/fidelity_board.rs` compares the two.
//!
//! Coordinates are scene coordinates: CSS pixels from the canvas's top
//! left corner, as `clientX - getBoundingClientRect().left` gives them.

use roughjs::{Options, RoughCanvas, RoughCanvasContext};

/// The drawing tools, in toolbar order, with their keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Selection,
    Rectangle,
    Diamond,
    Ellipse,
    Line,
}

impl Tool {
    pub const ALL: [Tool; 5] = [
        Tool::Selection,
        Tool::Rectangle,
        Tool::Diamond,
        Tool::Ellipse,
        Tool::Line,
    ];

    /// The name the DOM uses (`data-tool`, `id="tool-…"`).
    pub fn name(self) -> &'static str {
        match self {
            Tool::Selection => "selection",
            Tool::Rectangle => "rectangle",
            Tool::Diamond => "diamond",
            Tool::Ellipse => "ellipse",
            Tool::Line => "line",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tool::Selection => "Selection — V or 1",
            Tool::Rectangle => "Rectangle — R or 2",
            Tool::Diamond => "Diamond — D or 3",
            Tool::Ellipse => "Ellipse — O or 4",
            Tool::Line => "Line — L or 5",
        }
    }

    /// The label on the tool's button.
    pub fn label(self) -> &'static str {
        match self {
            Tool::Selection => "V",
            Tool::Rectangle => "R",
            Tool::Diamond => "D",
            Tool::Ellipse => "O",
            Tool::Line => "L",
        }
    }

    /// The tool a key selects, as `KeyboardEvent.key` names it.
    pub fn for_key(key: &str) -> Option<Tool> {
        Some(match key {
            "v" | "V" | "1" => Tool::Selection,
            "r" | "R" | "2" => Tool::Rectangle,
            "d" | "D" | "3" => Tool::Diamond,
            "o" | "O" | "4" => Tool::Ellipse,
            "l" | "L" | "5" => Tool::Line,
            _ => return None,
        })
    }

    /// The element type this tool creates; none for the selection tool.
    pub fn creates(self) -> Option<&'static str> {
        match self {
            Tool::Selection => None,
            other => Some(other.name()),
        }
    }
}

/// The style new elements get, and the style panel shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemStyle {
    pub stroke_color: String,
    pub background_color: String,
    pub stroke_width: f64,
    pub roughness: f64,
    pub fill_style: String,
}

impl Default for ItemStyle {
    fn default() -> Self {
        Self {
            stroke_color: "#1e1e1e".into(),
            background_color: "transparent".into(),
            stroke_width: 2.0,
            roughness: 1.0,
            fill_style: "solid".into(),
        }
    }
}

/// One style property and its value, as a style panel button sets it.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleChange {
    StrokeColor(String),
    BackgroundColor(String),
    StrokeWidth(f64),
    Roughness(f64),
    FillStyle(String),
}

/// The style panel: each section's DOM name, label and options. An
/// option's value is its `data-value`.
pub struct StyleSection {
    pub name: &'static str,
    pub label: &'static str,
    pub options: &'static [(&'static str, &'static str)],
}

/// (value, label)
pub const STYLE_SECTIONS: [StyleSection; 5] = [
    StyleSection {
        name: "stroke",
        label: "Stroke",
        options: &[
            ("#1e1e1e", ""),
            ("#e03131", ""),
            ("#2f9e44", ""),
            ("#1971c2", ""),
        ],
    },
    StyleSection {
        name: "background",
        label: "Background",
        options: &[
            ("transparent", ""),
            ("#ffc9c9", ""),
            ("#b2f2bb", ""),
            ("#a5d8ff", ""),
        ],
    },
    StyleSection {
        name: "width",
        label: "Stroke width",
        options: &[("1", "S"), ("2", "M"), ("4", "L")],
    },
    StyleSection {
        name: "roughness",
        label: "Sloppiness",
        options: &[("0", "0"), ("1", "1"), ("2", "2")],
    },
    StyleSection {
        name: "fill",
        label: "Fill",
        options: &[("hachure", "H"), ("cross-hatch", "X"), ("solid", "S")],
    },
];

impl StyleChange {
    /// The change a style panel button makes.
    pub fn from_option(section: &str, value: &str) -> StyleChange {
        let number = || value.parse::<f64>().expect("a number");
        match section {
            "stroke" => StyleChange::StrokeColor(value.into()),
            "background" => StyleChange::BackgroundColor(value.into()),
            "width" => StyleChange::StrokeWidth(number()),
            "roughness" => StyleChange::Roughness(number()),
            "fill" => StyleChange::FillStyle(value.into()),
            other => panic!("unknown style section {other}"),
        }
    }
}

impl ItemStyle {
    fn apply(&mut self, change: &StyleChange) {
        match change {
            StyleChange::StrokeColor(v) => self.stroke_color = v.clone(),
            StyleChange::BackgroundColor(v) => self.background_color = v.clone(),
            StyleChange::StrokeWidth(v) => self.stroke_width = *v,
            StyleChange::Roughness(v) => self.roughness = *v,
            StyleChange::FillStyle(v) => self.fill_style = v.clone(),
        }
    }

    /// The `data-value` of the option this style has in `section`.
    pub fn value(&self, section: &str) -> String {
        match section {
            "stroke" => self.stroke_color.clone(),
            "background" => self.background_color.clone(),
            "width" => roughjs::js::number_to_string(self.stroke_width),
            "roughness" => roughjs::js::number_to_string(self.roughness),
            "fill" => self.fill_style.clone(),
            other => panic!("unknown style section {other}"),
        }
    }
}

/// An element on the board.
///
/// For a rectangle, diamond or ellipse, (`x`, `y`) is the top left corner
/// and the size is never negative. For a line, (`x`, `y`) is its start and
/// (`width`, `height`) the offset to its end, which may be negative.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardElement {
    pub id: String,
    pub kind: &'static str,
    /// The label in the layers list, e.g. "Rectangle 1".
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub style: ItemStyle,
    pub seed: f64,
}

impl BoardElement {
    /// Whether the scene point (`x`, `y`) is inside the element's bounds,
    /// edges included.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (x0, x1) = ordered(self.x, self.x + self.width);
        let (y0, y1) = ordered(self.y, self.y + self.height);
        x0 <= x && x <= x1 && y0 <= y && y <= y1
    }

    /// The rough.js options the element is drawn with.
    pub fn rough_options(&self) -> Options {
        let s = &self.style;
        Options {
            seed: Some(self.seed),
            stroke: Some(s.stroke_color.clone()),
            stroke_width: Some(s.stroke_width),
            roughness: Some(s.roughness),
            hachure_gap: Some(s.stroke_width * 4.0),
            fill_weight: Some(s.stroke_width / 2.0),
            fill: (s.background_color != "transparent").then(|| s.background_color.clone()),
            fill_style: Some(s.fill_style.clone()),
            ..Options::default()
        }
    }

    /// Draws the element with rough.js.
    pub fn draw<C: RoughCanvasContext>(&self, rc: &mut RoughCanvas<C>) {
        let o = Some(self.rough_options());
        let o = o.as_ref();
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        match self.kind {
            "rectangle" => drop(rc.rectangle(x, y, w, h, o)),
            "ellipse" => drop(rc.ellipse(x + w / 2.0, y + h / 2.0, w, h, o)),
            "diamond" => drop(rc.polygon(
                &[
                    [x + w / 2.0, y],
                    [x + w, y + h / 2.0],
                    [x + w / 2.0, y + h],
                    [x, y + h / 2.0],
                ],
                o,
            )),
            "line" => drop(rc.linear_path(&[[x, y], [x + w, y + h]], o)),
            other => panic!("unknown element type {other}"),
        }
    }
}

fn ordered(a: f64, b: f64) -> (f64, f64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// A pointer gesture in progress.
#[derive(Debug, Clone, PartialEq)]
pub enum Drag {
    /// Creating `id` from the point where the pointer went down.
    Create { id: String, origin: (f64, f64) },
    /// Moving `id`; the pointer went down at `pointer` when the element was
    /// at `start`.
    Move {
        id: String,
        pointer: (f64, f64),
        start: (f64, f64),
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Bottom to top.
    pub elements: Vec<BoardElement>,
    pub selected: Option<String>,
    pub tool: Tool,
    pub style: ItemStyle,
    pub drag: Option<Drag>,
    /// The number of elements created so far: the next element is
    /// `el-{created + 1}`, with seed `created + 1`.
    pub created: u32,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            selected: None,
            tool: Tool::Selection,
            style: ItemStyle::default(),
            drag: None,
            created: 0,
        }
    }
}

impl Scene {
    pub fn element(&self, id: &str) -> Option<&BoardElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    fn element_mut(&mut self, id: &str) -> Option<&mut BoardElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    /// The topmost element at a scene point.
    pub fn hit(&self, x: f64, y: f64) -> Option<&BoardElement> {
        self.elements.iter().rev().find(|e| e.contains(x, y))
    }

    /// A tool button or shortcut: it clears the selection.
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.selected = None;
    }

    /// A style panel button: it sets the style for new elements and
    /// restyles the selected element.
    pub fn set_style(&mut self, change: &StyleChange) {
        self.style.apply(change);
        if let Some(id) = self.selected.clone() {
            if let Some(element) = self.element_mut(&id) {
                element.style.apply(change);
            }
        }
    }

    /// A row of the layers list: it selects that element.
    pub fn select(&mut self, id: &str) {
        if self.element(id).is_some() {
            self.tool = Tool::Selection;
            self.selected = Some(id.into());
        }
    }

    /// The primary button went down on the canvas.
    pub fn pointer_down(&mut self, x: f64, y: f64) {
        match self.tool.creates() {
            None => match self.hit(x, y) {
                Some(hit) => {
                    let id = hit.id.clone();
                    let start = (hit.x, hit.y);
                    self.selected = Some(id.clone());
                    self.drag = Some(Drag::Move {
                        id,
                        pointer: (x, y),
                        start,
                    });
                }
                None => self.selected = None,
            },
            Some(kind) => {
                self.created += 1;
                let n = self.created;
                let id = format!("el-{n}");
                let mut name = kind.to_string();
                name[..1].make_ascii_uppercase();
                self.elements.push(BoardElement {
                    id: id.clone(),
                    kind,
                    name: format!("{name} {n}"),
                    x,
                    y,
                    width: 0.0,
                    height: 0.0,
                    style: self.style.clone(),
                    seed: f64::from(n),
                });
                self.selected = Some(id.clone());
                self.drag = Some(Drag::Create { id, origin: (x, y) });
            }
        }
    }

    /// The pointer moved. Returns whether the scene changed.
    pub fn pointer_move(&mut self, x: f64, y: f64) -> bool {
        match self.drag.clone() {
            Some(Drag::Create { id, origin }) => {
                let element = self.element_mut(&id).expect("the new element");
                if element.kind == "line" {
                    element.width = x - origin.0;
                    element.height = y - origin.1;
                } else {
                    element.x = origin.0.min(x);
                    element.y = origin.1.min(y);
                    element.width = (x - origin.0).abs();
                    element.height = (y - origin.1).abs();
                }
                true
            }
            Some(Drag::Move { id, pointer, start }) => {
                let element = self.element_mut(&id).expect("the moved element");
                element.x = start.0 + (x - pointer.0);
                element.y = start.1 + (y - pointer.1);
                true
            }
            None => false,
        }
    }

    /// The primary button went up. A new element of no size is dropped.
    /// Returns whether the scene changed.
    pub fn pointer_up(&mut self) -> bool {
        match self.drag.take() {
            Some(Drag::Create { id, .. }) => {
                let element = self.element(&id).expect("the new element");
                if element.width == 0.0 && element.height == 0.0 {
                    self.elements.retain(|e| e.id != id);
                    self.selected = None;
                } else {
                    self.tool = Tool::Selection;
                }
                true
            }
            Some(Drag::Move { .. }) => true,
            None => false,
        }
    }

    /// A key, as `KeyboardEvent.key` names it. Returns whether the scene
    /// changed.
    pub fn key(&mut self, key: &str) -> bool {
        if self.drag.is_some() {
            return false;
        }
        if let Some(tool) = Tool::for_key(key) {
            self.set_tool(tool);
            return true;
        }
        match key {
            "Escape" => {
                self.selected = None;
                self.tool = Tool::Selection;
                true
            }
            "Delete" | "Backspace" => match self.selected.take() {
                Some(id) => {
                    self.elements.retain(|e| e.id != id);
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    /// Whether the style panel shows: while a creating tool is active or an
    /// element is selected.
    pub fn shows_style_panel(&self) -> bool {
        self.tool != Tool::Selection || self.selected.is_some()
    }

    /// Draws the scene, bottom to top. An element of no size is not drawn.
    pub fn draw<C: RoughCanvasContext>(&self, rc: &mut RoughCanvas<C>) {
        for element in &self.elements {
            if element.width != 0.0 || element.height != 0.0 {
                element.draw(rc);
            }
        }
    }
}
