//! A port of Konva's node tree: `Node`, `Container`, `Stage`, `Layer`,
//! `Group`, `Shape`, `Rect` and `Circle` (`Node.ts`, `Container.ts`,
//! `Shape.ts`, `shapes/Rect.ts`, `shapes/Circle.ts`).
//!
//! It covers the scene graph: attributes and their change events,
//! children and z-order, transforms and absolute transforms, positions,
//! client rects, visibility, listening, opacity, and Konva's own event
//! system with bubbling. Drawing, hit detection and dragging are not
//! ported yet (they need a 2D context, W2, and the hit canvas, W7).
//!
//! Konva caches transforms, visibility and similar derived values; the
//! port computes them each time. The cached and uncached paths of Konva
//! perform the same arithmetic in the same order, so results agree.

use crate::js;
use crate::transform::{IRect, Transform, Vector2d, PI_OVER_180};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

/// An attribute value. Konva stores any JavaScript value; the port
/// supports the scalar ones its node attributes use.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.into())
    }
}

impl Value {
    /// JavaScript truthiness.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Number(n) => js::truthy(*n),
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
        }
    }

    /// `===`.
    fn strict_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }
}

/// `nodeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Stage,
    Layer,
    Group,
    Shape,
}

/// `className`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassName {
    Stage,
    Layer,
    Group,
    Rect,
    Circle,
}

impl ClassName {
    pub fn node_type(self) -> NodeType {
        match self {
            ClassName::Stage => NodeType::Stage,
            ClassName::Layer => NodeType::Layer,
            ClassName::Group => NodeType::Group,
            ClassName::Rect | ClassName::Circle => NodeType::Shape,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ClassName::Stage => "Stage",
            ClassName::Layer => "Layer",
            ClassName::Group => "Group",
            ClassName::Rect => "Rect",
            ClassName::Circle => "Circle",
        }
    }
}

/// The event object handlers receive. One object travels the whole
/// bubbling path, as in Konva, so `cancel_bubble` set by a handler stops
/// it.
#[derive(Debug, Clone, Default)]
pub struct KonvaEvent {
    pub event_type: String,
    pub target: Option<Node>,
    pub current_target: Option<Node>,
    pub cancel_bubble: bool,
    /// For `<attr>Change`.
    pub old_val: Option<Value>,
    pub new_val: Option<Value>,
    /// For `add`.
    pub child: Option<Node>,
}

pub type Handler = Rc<dyn Fn(&Node, &mut KonvaEvent)>;

#[derive(Clone)]
struct Listener {
    name: String,
    handler: Handler,
}

/// The non-bubbling events (`NON_BUBBLING_EVENTS`).
const NON_BUBBLING: [&str; 6] = [
    "mouseenter",
    "mouseleave",
    "pointerenter",
    "pointerleave",
    "touchenter",
    "touchleave",
];

thread_local! {
    static ID_COUNTER: Cell<u64> = const { Cell::new(1) };
}

struct NodeData {
    id: u64,
    class: ClassName,
    /// `attrs`, in insertion order as a JavaScript object keeps it.
    attrs: RefCell<Vec<(String, Value)>>,
    parent: RefCell<Weak<NodeData>>,
    children: RefCell<Vec<Node>>,
    index: Cell<usize>,
    listeners: RefCell<BTreeMap<String, Vec<Listener>>>,
    should_fire_change_events: Cell<bool>,
}

/// A Konva node. Cloning the handle does not clone the node.
#[derive(Clone)]
pub struct Node(Rc<NodeData>);

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.0.class.as_str(), self.0.id)
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// `getClientRect(config)`.
#[derive(Debug, Clone, Default)]
pub struct ClientRectConfig {
    pub skip_transform: bool,
    pub skip_shadow: bool,
    pub skip_stroke: bool,
    pub relative_to: Option<Node>,
}

/// `transformsEnabled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformsEnabled {
    All,
    Position,
    None,
}

impl Node {
    /// `new Konva.<Class>(config)`. The attributes are set without change
    /// events, as in Konva's constructor.
    pub fn new(class: ClassName, config: &[(&str, Value)]) -> Node {
        let id = ID_COUNTER.with(|c| {
            let id = c.get();
            c.set(id + 1);
            id
        });
        let node = Node(Rc::new(NodeData {
            id,
            class,
            attrs: RefCell::new(Vec::new()),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(Vec::new()),
            index: Cell::new(0),
            listeners: RefCell::new(BTreeMap::new()),
            should_fire_change_events: Cell::new(false),
        }));
        node.set_attrs(config);
        node.0.should_fire_change_events.set(true);
        node
    }

    /// `_id`.
    pub fn internal_id(&self) -> u64 {
        self.0.id
    }

    pub fn class_name(&self) -> ClassName {
        self.0.class
    }

    pub fn node_type(&self) -> NodeType {
        self.0.class.node_type()
    }

    fn is_container(&self) -> bool {
        self.node_type() != NodeType::Shape
    }

    // ---- Attributes ----

    /// `attrs[key]`.
    pub fn attr(&self, key: &str) -> Option<Value> {
        self.0
            .attrs
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// `getAttrs()`, in key order.
    pub fn attrs(&self) -> Vec<(String, Value)> {
        self.0.attrs.borrow().clone()
    }

    fn number(&self, key: &str, default: f64) -> f64 {
        match self.attr(key) {
            Some(Value::Number(n)) => n,
            None => default,
            Some(other) => panic!("{key} is {other:?}, not a number"),
        }
    }

    fn flag(&self, key: &str, default: bool) -> bool {
        match self.attr(key) {
            Some(Value::Bool(b)) => b,
            None => default,
            Some(other) => panic!("{key} is {other:?}, not a boolean"),
        }
    }

    fn string(&self, key: &str) -> Option<String> {
        match self.attr(key) {
            Some(Value::String(s)) => Some(s),
            None => None,
            Some(other) => panic!("{key} is {other:?}, not a string"),
        }
    }

    /// `attrs.<key> || 0`: a missing or falsy value reads as 0.
    fn number_or_zero(&self, key: &str) -> f64 {
        match self.attr(key) {
            Some(Value::Number(n)) if js::truthy(n) => n,
            _ => 0.0,
        }
    }

    /// Writes `attrs[key]` directly, without events, as Konva does in
    /// `_clearTransform` and `_setTransform`.
    fn write_attr(&self, key: &str, value: Value) {
        let mut attrs = self.0.attrs.borrow_mut();
        match attrs.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => attrs.push((key.into(), value)),
        }
    }

    /// `_setAttr(key, val)`.
    fn set_attr_raw(&self, key: &str, value: Value) {
        let old = self.attr(key);
        if old.as_ref().is_some_and(|old| old.strict_eq(&value)) {
            return;
        }
        self.write_attr(key, value.clone());
        if self.0.should_fire_change_events.get() {
            let mut evt = KonvaEvent {
                old_val: old,
                new_val: Some(value),
                ..KonvaEvent::default()
            };
            self.fire_own(&format!("{key}Change"), &mut evt);
        }
    }

    /// `setAttr(key, val)`: the attribute's setter, if it has its own.
    pub fn set_attr(&self, key: &str, value: Value) {
        let number = || match &value {
            Value::Number(n) => *n,
            other => panic!("{key} takes a number, not {other:?}"),
        };
        match (self.class_name(), key) {
            (_, "zIndex") => self.set_z_index(number() as i64),
            // Circle's width and height setters set the radius.
            (ClassName::Circle, "width" | "height") => {
                self.set_attr_raw("radius", Value::Number(number() / 2.0))
            }
            // A layer is as large as its stage; Konva only warns.
            (ClassName::Layer, "width" | "height") => {}
            _ => self.set_attr_raw(key, value),
        }
    }

    /// `setAttrs(config)`.
    pub fn set_attrs(&self, config: &[(&str, Value)]) {
        for (key, value) in config {
            if *key == "children" {
                continue;
            }
            self.set_attr(key, value.clone());
        }
    }

    pub fn x(&self) -> f64 {
        self.number("x", 0.0)
    }
    pub fn y(&self) -> f64 {
        self.number("y", 0.0)
    }
    pub fn rotation(&self) -> f64 {
        self.number("rotation", 0.0)
    }
    pub fn scale_x(&self) -> f64 {
        self.number("scaleX", 1.0)
    }
    pub fn scale_y(&self) -> f64 {
        self.number("scaleY", 1.0)
    }
    pub fn skew_x(&self) -> f64 {
        self.number("skewX", 0.0)
    }
    pub fn skew_y(&self) -> f64 {
        self.number("skewY", 0.0)
    }
    pub fn offset_x(&self) -> f64 {
        self.number("offsetX", 0.0)
    }
    pub fn offset_y(&self) -> f64 {
        self.number("offsetY", 0.0)
    }
    pub fn opacity(&self) -> f64 {
        self.number("opacity", 1.0)
    }
    pub fn visible(&self) -> bool {
        self.flag("visible", true)
    }
    pub fn listening(&self) -> bool {
        self.flag("listening", true)
    }
    pub fn transforms_enabled(&self) -> TransformsEnabled {
        match self.string("transformsEnabled").as_deref() {
            None | Some("all") => TransformsEnabled::All,
            Some("position") => TransformsEnabled::Position,
            Some(_) => TransformsEnabled::None,
        }
    }

    /// `width()`: a circle's is twice its radius, and a layer's is its
    /// stage's. A layer without a stage has no width in Konva
    /// (`undefined`); the port returns NaN for it.
    pub fn width(&self) -> f64 {
        match self.class_name() {
            ClassName::Circle => self.number("radius", 0.0).abs() * 2.0,
            ClassName::Layer => self.parent().map_or(f64::NAN, |p| p.width()),
            _ => self.number("width", 0.0),
        }
    }

    /// `height()`, as `width()`.
    pub fn height(&self) -> f64 {
        match self.class_name() {
            ClassName::Circle => self.number("radius", 0.0).abs() * 2.0,
            ClassName::Layer => self.parent().map_or(f64::NAN, |p| p.height()),
            _ => self.number("height", 0.0),
        }
    }

    pub fn position(&self) -> Vector2d {
        Vector2d {
            x: self.x(),
            y: self.y(),
        }
    }

    /// `setPosition({x, y})`.
    pub fn set_position(&self, pos: Vector2d) {
        self.set_attr("x", pos.x.into());
        self.set_attr("y", pos.y.into());
    }

    /// `move({x, y})`.
    pub fn move_by(&self, dx: Option<f64>, dy: Option<f64>) {
        let (mut x, mut y) = (self.x(), self.y());
        if let Some(dx) = dx {
            x += dx;
        }
        if let Some(dy) = dy {
            y += dy;
        }
        self.set_position(Vector2d { x, y });
    }

    /// `rotate(theta)`, in degrees.
    pub fn rotate(&self, theta: f64) {
        self.set_attr("rotation", (self.rotation() + theta).into());
    }

    // ---- Tree ----

    pub fn parent(&self) -> Option<Node> {
        self.0.parent.borrow().upgrade().map(Node)
    }

    /// `getChildren()`.
    pub fn children(&self) -> Vec<Node> {
        self.0.children.borrow().clone()
    }

    /// `index`, the node's place among its siblings.
    pub fn z_index(&self) -> usize {
        self.0.index.get()
    }

    fn set_children_indices(&self) {
        for (i, child) in self.0.children.borrow().iter().enumerate() {
            child.0.index.set(i);
        }
    }

    /// `getLayer()`.
    pub fn layer(&self) -> Option<Node> {
        match self.node_type() {
            NodeType::Layer => Some(self.clone()),
            NodeType::Stage => None,
            _ => self.parent().and_then(|p| p.layer()),
        }
    }

    /// `getStage()`.
    pub fn stage(&self) -> Option<Node> {
        match self.node_type() {
            NodeType::Stage => Some(self.clone()),
            _ => self.parent().and_then(|p| p.stage()),
        }
    }

    /// `isAncestorOf(node)`: false for a shape.
    pub fn is_ancestor_of(&self, node: &Node) -> bool {
        if !self.is_container() {
            return false;
        }
        let mut parent = node.parent();
        while let Some(p) = parent {
            if p == *self {
                return true;
            }
            parent = p.parent();
        }
        false
    }

    /// `getDepth()`.
    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut parent = self.parent();
        while let Some(p) = parent {
            depth += 1;
            parent = p.parent();
        }
        depth
    }

    /// `getAbsoluteZIndex()`.
    pub fn absolute_z_index(&self) -> usize {
        let depth = self.depth();
        let mut index = 0;
        fn add_children(children: Vec<Node>, me: &Node, depth: usize, index: &mut usize) {
            let mut nodes = Vec::new();
            for child in &children {
                *index += 1;
                if child.node_type() != NodeType::Shape {
                    nodes.extend(child.children());
                }
                if child == me {
                    break;
                }
            }
            if !nodes.is_empty() && nodes[0].depth() <= depth {
                add_children(nodes, me, depth, index);
            }
        }
        if self.node_type() != NodeType::Stage {
            if let Some(stage) = self.stage() {
                add_children(stage.children(), self, depth, &mut index);
            }
        }
        index
    }

    /// `add(child)`. Panics where Konva throws: a stage takes layers only,
    /// and a layer or group takes groups and shapes only.
    pub fn add(&self, child: &Node) {
        assert!(self.is_container(), "a shape has no children");
        if child.parent().is_some() {
            child.move_to(self);
            return;
        }
        let ok = match self.node_type() {
            NodeType::Stage => child.node_type() == NodeType::Layer,
            _ => matches!(child.node_type(), NodeType::Group | NodeType::Shape),
        };
        assert!(
            ok,
            "Konva throws: cannot add a {:?} to a {:?}",
            child.node_type(),
            self.node_type()
        );
        child.0.index.set(self.0.children.borrow().len());
        *child.0.parent.borrow_mut() = Rc::downgrade(&self.0);
        self.0.children.borrow_mut().push(child.clone());
        let mut evt = KonvaEvent {
            child: Some(child.clone()),
            ..KonvaEvent::default()
        };
        self.fire_own("add", &mut evt);
    }

    /// `_remove()`.
    fn detach(&self) {
        if let Some(parent) = self.parent() {
            parent.0.children.borrow_mut().remove(self.z_index());
            parent.set_children_indices();
            *self.0.parent.borrow_mut() = Weak::new();
        }
    }

    /// `remove()`.
    pub fn remove(&self) {
        self.detach();
    }

    /// `destroy()`: a container destroys its children first.
    pub fn destroy(&self) {
        if self.is_container() {
            for child in self.children() {
                *child.0.parent.borrow_mut() = Weak::new();
                child.0.index.set(0);
                child.destroy();
            }
            self.0.children.borrow_mut().clear();
        }
        let mut evt = KonvaEvent {
            target: Some(self.clone()),
            ..KonvaEvent::default()
        };
        self.fire_own("destroy", &mut evt);
        self.remove();
    }

    /// `moveTo(container)`.
    pub fn move_to(&self, container: &Node) {
        if self.parent().as_ref() != Some(container) {
            self.detach();
            container.add(self);
        }
    }

    fn reorder(&self, f: impl FnOnce(&mut Vec<Node>, usize) -> bool) -> bool {
        let Some(parent) = self.parent() else {
            return false;
        };
        let moved = f(&mut parent.0.children.borrow_mut(), self.z_index());
        if moved {
            parent.set_children_indices();
        }
        moved
    }

    pub fn move_to_top(&self) -> bool {
        self.reorder(|children, index| {
            if index < children.len() - 1 {
                let me = children.remove(index);
                children.push(me);
                true
            } else {
                false
            }
        })
    }

    pub fn move_up(&self) -> bool {
        self.reorder(|children, index| {
            if index < children.len() - 1 {
                let me = children.remove(index);
                children.insert(index + 1, me);
                true
            } else {
                false
            }
        })
    }

    pub fn move_down(&self) -> bool {
        self.reorder(|children, index| {
            if index > 0 {
                let me = children.remove(index);
                children.insert(index - 1, me);
                true
            } else {
                false
            }
        })
    }

    pub fn move_to_bottom(&self) -> bool {
        self.reorder(|children, index| {
            if index > 0 {
                let me = children.remove(index);
                children.insert(0, me);
                true
            } else {
                false
            }
        })
    }

    /// `setZIndex(z)`. An index out of range is clamped by
    /// `Array.prototype.splice`, as in Konva (which also warns).
    pub fn set_z_index(&self, z: i64) {
        self.reorder(|children, index| {
            let me = children.remove(index);
            // splice(start): a negative start counts from the end.
            let len = children.len() as i64;
            let at = if z < 0 { (len + z).max(0) } else { z.min(len) };
            children.insert(at as usize, me);
            true
        });
    }

    // ---- Visibility, listening, opacity ----

    /// `isVisible()`.
    pub fn is_visible(&self) -> bool {
        self.visible() && self.parent().is_none_or(|p| p.is_visible())
    }

    /// `isListening()`.
    pub fn is_listening(&self) -> bool {
        self.listening() && self.parent().is_none_or(|p| p.is_listening())
    }

    /// `getAbsoluteOpacity()`.
    pub fn absolute_opacity(&self) -> f64 {
        let mut opacity = self.opacity();
        if let Some(parent) = self.parent() {
            opacity *= parent.absolute_opacity();
        }
        opacity
    }

    // ---- Transforms ----

    /// `getTransform()`.
    pub fn transform(&self) -> Transform {
        let mut m = Transform::new();
        let (x, y) = (self.x(), self.y());
        let rotation = self.rotation() * PI_OVER_180;
        // `attrs.scaleX ?? 1`
        let scale_x = self.number("scaleX", 1.0);
        let scale_y = self.number("scaleY", 1.0);
        let skew_x = self.number_or_zero("skewX");
        let skew_y = self.number_or_zero("skewY");
        let offset_x = self.number_or_zero("offsetX");
        let offset_y = self.number_or_zero("offsetY");
        if x != 0.0 || y != 0.0 {
            m.translate(x, y);
        }
        if rotation != 0.0 {
            m.rotate(rotation);
        }
        if skew_x != 0.0 || skew_y != 0.0 {
            m.skew(skew_x, skew_y);
        }
        if scale_x != 1.0 || scale_y != 1.0 {
            m.scale(scale_x, scale_y);
        }
        if offset_x != 0.0 || offset_y != 0.0 {
            m.translate(-offset_x, -offset_y);
        }
        m
    }

    fn multiply_own_transform(&self, at: &mut Transform) {
        match self.transforms_enabled() {
            TransformsEnabled::All => {
                at.multiply(&self.transform());
            }
            TransformsEnabled::Position => {
                let x = self.number_or_zero("x");
                let y = self.number_or_zero("y");
                let offset_x = self.number_or_zero("offsetX");
                let offset_y = self.number_or_zero("offsetY");
                at.translate(x - offset_x, y - offset_y);
            }
            TransformsEnabled::None => {}
        }
    }

    /// `getAbsoluteTransform(top)`: the transforms from `top` (exclusive)
    /// down to this node.
    pub fn absolute_transform(&self, top: Option<&Node>) -> Transform {
        let mut at = Transform::new();
        if top == Some(self) {
            return at;
        }
        if let Some(parent) = self.parent() {
            if Some(&parent) != top {
                at = parent.absolute_transform(top);
            }
        }
        self.multiply_own_transform(&mut at);
        at
    }

    /// `getAbsolutePosition(top)`.
    pub fn absolute_position(&self, top: Option<&Node>) -> Vector2d {
        let mut at = self.absolute_transform(top);
        at.translate(self.offset_x(), self.offset_y());
        at.translation()
    }

    /// `setAbsolutePosition(pos)`.
    pub fn set_absolute_position(&self, pos: Vector2d) {
        // _clearTransform: read every transform attribute, then reset them
        // in attrs directly.
        let orig = [
            ("rotation", self.rotation()),
            ("scaleX", self.scale_x()),
            ("scaleY", self.scale_y()),
            ("offsetX", self.offset_x()),
            ("offsetY", self.offset_y()),
            ("skewX", self.skew_x()),
            ("skewY", self.skew_y()),
        ];
        let (x, y) = (self.x(), self.y());
        for (key, value) in [
            ("x", 0.0),
            ("y", 0.0),
            ("rotation", 0.0),
            ("scaleX", 1.0),
            ("scaleY", 1.0),
            ("offsetX", 0.0),
            ("offsetY", 0.0),
            ("skewX", 0.0),
            ("skewY", 0.0),
        ] {
            self.write_attr(key, Value::Number(value));
        }
        self.write_attr("x", Value::Number(x));
        self.write_attr("y", Value::Number(y));
        let mut it = self.absolute_transform(None);
        let invertible = it.is_invertible();
        it.invert();
        it.translate(pos.x, pos.y);
        let d = it.translation();
        for (key, value) in orig {
            self.write_attr(key, Value::Number(value));
        }
        if invertible {
            let (ax, ay) = (self.number("x", 0.0), self.number("y", 0.0));
            self.set_position(Vector2d {
                x: ax + d.x,
                y: ay + d.y,
            });
        }
    }

    /// `getAbsoluteScale()`.
    pub fn absolute_scale(&self) -> Vector2d {
        let d = self.absolute_transform(None).decompose();
        Vector2d {
            x: d.scale_x,
            y: d.scale_y,
        }
    }

    /// `getAbsoluteRotation()`, in degrees.
    pub fn absolute_rotation(&self) -> f64 {
        self.absolute_transform(None).decompose().rotation
    }

    fn transformed_rect(&self, rect: IRect, top: Option<&Node>) -> IRect {
        self.absolute_transform(top).transformed_rect(rect)
    }

    // ---- Client rects ----

    /// `getSelfRect()`: a circle is centred on its position.
    pub fn self_rect(&self) -> IRect {
        let (width, height) = (self.width(), self.height());
        let centroid = self.class_name() == ClassName::Circle;
        IRect {
            x: if centroid { -width / 2.0 } else { 0.0 },
            y: if centroid { -height / 2.0 } else { 0.0 },
            width,
            height,
        }
    }

    pub fn stroke_width(&self) -> f64 {
        self.number("strokeWidth", 2.0)
    }

    /// `hasStroke()`.
    pub fn has_stroke(&self) -> bool {
        self.flag("strokeEnabled", true)
            && js::truthy(self.stroke_width())
            && self.attr("stroke").is_some_and(|s| s.truthy())
    }

    /// `hasShadow()`.
    pub fn has_shadow(&self) -> bool {
        self.flag("shadowEnabled", true)
            && self.number("shadowOpacity", 1.0) != 0.0
            && (self.attr("shadowColor").is_some_and(|s| s.truthy())
                || js::truthy(self.number("shadowBlur", 0.0))
                || js::truthy(self.number("shadowOffsetX", 0.0))
                || js::truthy(self.number("shadowOffsetY", 0.0)))
    }

    /// `getClientRect(config)`.
    pub fn client_rect(&self, config: &ClientRectConfig) -> IRect {
        if self.is_container() {
            self.container_client_rect(config)
        } else {
            self.shape_client_rect(config)
        }
    }

    fn shape_client_rect(&self, config: &ClientRectConfig) -> IRect {
        let skip_transform = config.skip_transform;
        let relative_to = config.relative_to.as_ref();
        let mut fill_rect = self.self_rect();
        if fill_rect.width < 0.0 || fill_rect.height < 0.0 {
            fill_rect = Transform::new().transformed_rect(fill_rect);
        }
        let apply_stroke = !config.skip_stroke && self.has_stroke();
        let stroke_width = if apply_stroke {
            self.stroke_width()
        } else {
            0.0
        };
        let (mut stroke_width_x, mut stroke_width_y) = (stroke_width, stroke_width);
        let mut collapsed_stroke: Option<Transform> = None;
        let stroke_scale_enabled = self.flag("strokeScaleEnabled", true);
        if js::truthy(stroke_width) && !stroke_scale_enabled {
            // No ancestor is cached in the port, so the stroke is constant
            // in the stage's space.
            let [a, b, c, d, _, _] = self.absolute_transform(None).m;
            let determinant = (a * d - b * c).abs();
            if js::truthy(determinant) {
                stroke_width_x *= js::hypot(&[c, d]) / determinant;
                stroke_width_y *= js::hypot(&[a, b]) / determinant;
            } else if !skip_transform {
                stroke_width_x = 0.0;
                stroke_width_y = 0.0;
                let mut collapsed = Transform::new();
                if let Some(relative_to) = relative_to {
                    let relative = relative_to.absolute_transform(None);
                    let [a, b, c, d, _, _] = relative.m;
                    collapsed = if js::truthy(a * d - b * c) {
                        let mut inverse = relative;
                        inverse.invert();
                        inverse.multiply(&collapsed);
                        inverse
                    } else {
                        Transform::new()
                    };
                }
                collapsed_stroke = Some(collapsed);
            }
        }
        let mut rect = IRect {
            x: fill_rect.x - stroke_width_x / 2.0,
            y: fill_rect.y - stroke_width_y / 2.0,
            width: fill_rect.width + stroke_width_x,
            height: fill_rect.height + stroke_width_y,
        };
        if !config.skip_shadow && self.has_shadow() {
            let dx = self.number("shadowOffsetX", 0.0);
            let dy = self.number("shadowOffsetY", 0.0);
            let blur = self.number("shadowBlur", 0.0);
            rect.x += js::min(&[0.0, dx]) - blur;
            rect.y += js::min(&[0.0, dy]) - blur;
            rect.width += dx.abs() + blur * 2.0;
            rect.height += dy.abs() + blur * 2.0;
        }
        if !skip_transform {
            let mut transformed = self.transformed_rect(rect, relative_to);
            if let Some(collapsed) = collapsed_stroke {
                let [a, b, c, d, _, _] = collapsed.m;
                let x = (stroke_width * js::hypot(&[a, c])) / 2.0;
                let y = (stroke_width * js::hypot(&[b, d])) / 2.0;
                transformed.x -= x;
                transformed.y -= y;
                transformed.width += x * 2.0;
                transformed.height += y * 2.0;
            }
            return transformed;
        }
        rect
    }

    fn container_client_rect(&self, config: &ClientRectConfig) -> IRect {
        let skip_transform = config.skip_transform;
        let relative_to = config.relative_to.as_ref();
        let [a, b, c, d, _, _] = self.absolute_transform(None).m;
        let measure_transformed = !skip_transform && a * d - b * c == 0.0;
        let mut bounds: Option<(f64, f64, f64, f64)> = None;
        for child in self.children() {
            if !child.visible() {
                continue;
            }
            let rect = child.client_rect(&ClientRectConfig {
                skip_transform: false,
                skip_shadow: config.skip_shadow,
                skip_stroke: config.skip_stroke,
                relative_to: if measure_transformed {
                    config.relative_to.clone()
                } else {
                    Some(self.clone())
                },
            });
            if (rect.width == 0.0 && rect.height == 0.0)
                || !(rect.x + rect.y + rect.width + rect.height).is_finite()
            {
                continue;
            }
            bounds = Some(match bounds {
                None => (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
                Some((min_x, min_y, max_x, max_y)) => (
                    js::min(&[min_x, rect.x]),
                    js::min(&[min_y, rect.y]),
                    js::max(&[max_x, rect.x + rect.width]),
                    js::max(&[max_y, rect.y + rect.height]),
                ),
            });
        }
        let self_rect = match bounds {
            Some((min_x, min_y, max_x, max_y)) => IRect {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            },
            None => IRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
        };
        if !skip_transform && !measure_transformed {
            return self.transformed_rect(self_rect, relative_to);
        }
        self_rect
    }

    // ---- Events ----

    /// `on(evtStr, handler)`: space-separated types, each with an optional
    /// `.name`.
    pub fn on(&self, events: &str, handler: Handler) {
        for event in events.split(' ') {
            let mut parts = event.split('.');
            let base = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("");
            self.0
                .listeners
                .borrow_mut()
                .entry(base.into())
                .or_default()
                .push(Listener {
                    name: name.into(),
                    handler: handler.clone(),
                });
        }
    }

    /// `off(evtStr)`: by type, by `.name`, or everything with `None`.
    pub fn off(&self, events: Option<&str>) {
        let types: Vec<String> = self.0.listeners.borrow().keys().cloned().collect();
        let Some(events) = events.filter(|e| !e.is_empty()) else {
            for t in &types {
                self.off_one(t, None);
            }
            // Konva then runs the loop below once with an empty event,
            // which removes nothing more.
            return;
        };
        for event in events.split(' ') {
            let mut parts = event.split('.');
            let base = parts.next().unwrap_or("");
            let name = parts.next();
            if !base.is_empty() {
                if self.0.listeners.borrow().contains_key(base) {
                    self.off_one(base, name);
                }
            } else {
                let types: Vec<String> = self.0.listeners.borrow().keys().cloned().collect();
                for t in &types {
                    self.off_one(t, name);
                }
            }
        }
    }

    /// `_off(type, name)`: a listener named `konva` is removed only by name.
    fn off_one(&self, event_type: &str, name: Option<&str>) {
        let mut listeners = self.0.listeners.borrow_mut();
        let Some(list) = listeners.get_mut(event_type) else {
            return;
        };
        let name = name.filter(|n| !n.is_empty());
        list.retain(|l| {
            let removable =
                (l.name != "konva" || name == Some("konva")) && name.is_none_or(|n| l.name == n);
            !removable
        });
        if list.is_empty() {
            listeners.remove(event_type);
        }
    }

    /// `fire(eventType, evt, bubble)`.
    pub fn fire(&self, event_type: &str, evt: &mut KonvaEvent, bubble: bool) {
        if evt.target.is_none() {
            evt.target = Some(self.clone());
        }
        if bubble {
            self.fire_and_bubble(event_type, evt, None);
        } else {
            self.fire_own(event_type, evt);
        }
    }

    /// `_fireAndBubble(eventType, evt, compareShape)`.
    pub fn fire_and_bubble(&self, event_type: &str, evt: &mut KonvaEvent, compare: Option<&Node>) {
        if self.node_type() == NodeType::Shape {
            evt.target = Some(self.clone());
        }
        let non_bubbling = NON_BUBBLING.contains(&event_type);
        let should_stop = non_bubbling
            && (compare.is_some_and(|c| c == self || self.is_ancestor_of(c))
                || (self.node_type() == NodeType::Stage && compare.is_none()));
        if should_stop {
            return;
        }
        self.fire_own(event_type, evt);
        let parent = self.parent();
        let stop_bubble = non_bubbling
            && compare.is_some_and(|c| {
                c.is_ancestor_of(self) && parent.as_ref().is_none_or(|p| !c.is_ancestor_of(p))
            });
        if let Some(parent) = parent {
            if !evt.cancel_bubble && parent.is_listening() && !stop_bubble {
                let compare = compare.filter(|c| c.parent().is_some());
                parent.fire_and_bubble(event_type, evt, compare);
            }
        }
    }

    /// `_fire(eventType, evt)`: this node's listeners, including any added
    /// while they run.
    fn fire_own(&self, event_type: &str, evt: &mut KonvaEvent) {
        evt.current_target = Some(self.clone());
        evt.event_type = event_type.into();
        let list: Vec<Listener> = match self.0.listeners.borrow().get(event_type) {
            Some(list) => list.clone(),
            None => return,
        };
        let original = list.len();
        for listener in &list {
            (listener.handler)(self, evt);
        }
        let added: Vec<Listener> = self
            .0
            .listeners
            .borrow()
            .get(event_type)
            .map(|live| live.iter().skip(original).cloned().collect())
            .unwrap_or_default();
        for listener in &added {
            (listener.handler)(self, evt);
        }
    }
}
