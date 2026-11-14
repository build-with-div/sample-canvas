//! The `CanvasRenderingContext2D` members Konva's drawing uses, and a
//! recorder of the calls.
//!
//! div-gap: W2 CanvasRenderingContext2D. Div has no 2D context yet, so the
//! port draws through `Context2d`, a trait naming those members with
//! their web arguments, and its only implementation is `CallRecorder`,
//! which records the calls and draws nothing. When Div's context lands it
//! implements `Context2d` (or replaces it) and the recorder is deleted.

use std::collections::HashMap;

/// A value set on a context property, or a call argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Number(f64),
    Str(String),
    Bool(bool),
    Numbers(Vec<f64>),
}

/// One recorded context call or property assignment.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasCall {
    Call {
        method: &'static str,
        args: Vec<Arg>,
    },
    Set {
        property: String,
        value: Arg,
    },
}

/// The members of `CanvasRenderingContext2D` that Konva's `Context`
/// forwards for rects and circles, by their web names.
pub trait Context2d {
    fn save(&mut self);
    fn restore(&mut self);
    fn begin_path(&mut self);
    fn close_path(&mut self);
    fn rect(&mut self, x: f64, y: f64, width: f64, height: f64);
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64, ccw: bool);
    fn move_to(&mut self, x: f64, y: f64);
    fn line_to(&mut self, x: f64, y: f64);
    /// `fill()` or `fill(fillRule)`.
    fn fill(&mut self, fill_rule: Option<&str>);
    fn stroke(&mut self);
    fn clip(&mut self);
    fn transform(&mut self, m: [f64; 6]);
    fn set_transform(&mut self, m: [f64; 6]);
    fn scale(&mut self, x: f64, y: f64);
    fn clear_rect(&mut self, x: f64, y: f64, width: f64, height: f64);
    fn set_line_dash(&mut self, segments: &[f64]);
    /// `context[property] = value`, as Konva's `setAttr` assigns it.
    fn set_property(&mut self, property: &str, value: Arg);
    /// Reads back `shadowColor`; Konva saves it around a stroke.
    fn shadow_color(&self) -> Arg;
    /// Reads back `imageSmoothingEnabled`; Konva keeps it across a resize.
    fn image_smoothing_enabled(&self) -> Arg;
}

/// Records every call and assignment, and draws nothing.
///
/// Reading a property gives back the value last assigned, or the initial
/// value Chromium reports. A real context also normalizes what it is
/// given (a `shadowColor` of `black` reads back as `#000000`) and resets
/// its state when the canvas is resized; the recorder does neither.
#[derive(Debug, Default)]
pub struct CallRecorder {
    calls: Vec<CanvasCall>,
    properties: HashMap<String, Arg>,
}

impl CallRecorder {
    /// The calls recorded so far, oldest first.
    pub fn calls(&self) -> &[CanvasCall] {
        &self.calls
    }

    /// Returns the recorded calls and starts a new record.
    pub fn take_calls(&mut self) -> Vec<CanvasCall> {
        std::mem::take(&mut self.calls)
    }

    fn call(&mut self, method: &'static str, args: Vec<Arg>) {
        self.calls.push(CanvasCall::Call { method, args });
    }

    fn numbers(&mut self, method: &'static str, args: &[f64]) {
        self.call(method, args.iter().map(|&v| Arg::Number(v)).collect());
    }

    fn property(&self, name: &str, initial: Arg) -> Arg {
        self.properties.get(name).cloned().unwrap_or(initial)
    }
}

impl Context2d for CallRecorder {
    fn save(&mut self) {
        self.numbers("save", &[]);
    }
    fn restore(&mut self) {
        self.numbers("restore", &[]);
    }
    fn begin_path(&mut self) {
        self.numbers("beginPath", &[]);
    }
    fn close_path(&mut self) {
        self.numbers("closePath", &[]);
    }
    fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.numbers("rect", &[x, y, width, height]);
    }
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64, ccw: bool) {
        let mut args: Vec<Arg> = [x, y, radius, start, end].map(Arg::Number).into();
        args.push(Arg::Bool(ccw));
        self.call("arc", args);
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.numbers("moveTo", &[x, y]);
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.numbers("lineTo", &[x, y]);
    }
    fn fill(&mut self, fill_rule: Option<&str>) {
        let args = fill_rule.map(|r| Arg::Str(r.into())).into_iter().collect();
        self.call("fill", args);
    }
    fn stroke(&mut self) {
        self.numbers("stroke", &[]);
    }
    fn clip(&mut self) {
        self.numbers("clip", &[]);
    }
    fn transform(&mut self, m: [f64; 6]) {
        self.numbers("transform", &m);
    }
    fn set_transform(&mut self, m: [f64; 6]) {
        self.numbers("setTransform", &m);
    }
    fn scale(&mut self, x: f64, y: f64) {
        self.numbers("scale", &[x, y]);
    }
    fn clear_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.numbers("clearRect", &[x, y, width, height]);
    }
    fn set_line_dash(&mut self, segments: &[f64]) {
        self.call("setLineDash", vec![Arg::Numbers(segments.to_vec())]);
    }
    fn set_property(&mut self, property: &str, value: Arg) {
        self.properties.insert(property.into(), value.clone());
        self.calls.push(CanvasCall::Set {
            property: property.into(),
            value,
        });
    }
    fn shadow_color(&self) -> Arg {
        self.property("shadowColor", Arg::Str("rgba(0, 0, 0, 0)".into()))
    }
    fn image_smoothing_enabled(&self) -> Arg {
        self.property("imageSmoothingEnabled", Arg::Bool(true))
    }
}
