//! A port of rough.js's `RoughCanvas` (`canvas.ts`): drawing a generator's
//! operations with a 2D canvas context.

use crate::core::{Drawable, OpSet, OpSetType, OpType, Options, Point, ResolvedOptions};
use crate::generator::RoughGenerator;
use crate::js;
use crate::path_data::PathDataError;

/// `CanvasFillRule`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasFillRule {
    NonZero,
    EvenOdd,
}

impl CanvasFillRule {
    pub fn as_str(self) -> &'static str {
        match self {
            CanvasFillRule::NonZero => "nonzero",
            CanvasFillRule::EvenOdd => "evenodd",
        }
    }
}

// div-gap: W2 CanvasRenderingContext2D. Div has no 2D context yet
// (docs/design.md §2, framework-gaps.md gap 1). This trait names the
// members of `CanvasRenderingContext2D` that rough.js's renderer calls,
// with their web names and arguments. It is an interface, not a context:
// its only implementation here is `CallRecorder`, which draws nothing.
// When Div's `CanvasRenderingContext2d` lands, `RoughCanvas` takes it
// directly and this trait is deleted.
pub trait RoughCanvasContext {
    fn save(&mut self);
    fn restore(&mut self);
    /// `ctx.strokeStyle = value`, a CSS color string.
    fn set_stroke_style(&mut self, value: &str);
    /// `ctx.fillStyle = value`, a CSS color string.
    fn set_fill_style(&mut self, value: &str);
    fn set_line_width(&mut self, value: f64);
    fn set_line_dash(&mut self, segments: &[f64]);
    fn set_line_dash_offset(&mut self, value: f64);
    fn begin_path(&mut self);
    fn move_to(&mut self, x: f64, y: f64);
    fn line_to(&mut self, x: f64, y: f64);
    fn bezier_curve_to(&mut self, cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64);
    fn fill(&mut self, rule: CanvasFillRule);
    fn stroke(&mut self);
}

/// One call on the context, as `CallRecorder` records it.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasCall {
    Save,
    Restore,
    SetStrokeStyle(String),
    SetFillStyle(String),
    SetLineWidth(f64),
    SetLineDash(Vec<f64>),
    SetLineDashOffset(f64),
    BeginPath,
    MoveTo(f64, f64),
    LineTo(f64, f64),
    BezierCurveTo([f64; 6]),
    Fill(CanvasFillRule),
    Stroke,
}

// div-gap: W2 CanvasRenderingContext2D. A recorder of the calls rough.js
// makes, used to compare the port's calls with the original's. It is
// measurement scaffolding, not a renderer.
#[derive(Debug, Clone, Default)]
pub struct CallRecorder {
    pub calls: Vec<CanvasCall>,
}

impl RoughCanvasContext for CallRecorder {
    fn save(&mut self) {
        self.calls.push(CanvasCall::Save);
    }
    fn restore(&mut self) {
        self.calls.push(CanvasCall::Restore);
    }
    fn set_stroke_style(&mut self, value: &str) {
        self.calls.push(CanvasCall::SetStrokeStyle(value.into()));
    }
    fn set_fill_style(&mut self, value: &str) {
        self.calls.push(CanvasCall::SetFillStyle(value.into()));
    }
    fn set_line_width(&mut self, value: f64) {
        self.calls.push(CanvasCall::SetLineWidth(value));
    }
    fn set_line_dash(&mut self, segments: &[f64]) {
        self.calls.push(CanvasCall::SetLineDash(segments.to_vec()));
    }
    fn set_line_dash_offset(&mut self, value: f64) {
        self.calls.push(CanvasCall::SetLineDashOffset(value));
    }
    fn begin_path(&mut self) {
        self.calls.push(CanvasCall::BeginPath);
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.calls.push(CanvasCall::MoveTo(x, y));
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.calls.push(CanvasCall::LineTo(x, y));
    }
    fn bezier_curve_to(&mut self, cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64) {
        self.calls
            .push(CanvasCall::BezierCurveTo([cp1x, cp1y, cp2x, cp2y, x, y]));
    }
    fn fill(&mut self, rule: CanvasFillRule) {
        self.calls.push(CanvasCall::Fill(rule));
    }
    fn stroke(&mut self) {
        self.calls.push(CanvasCall::Stroke);
    }
}

/// `new RoughCanvas(canvas, config)`.
///
/// The original takes a `<canvas>` element and calls `getContext('2d')`
/// itself. Div has neither yet (W1, W2), so the port takes the context.
pub struct RoughCanvas<C: RoughCanvasContext> {
    ctx: C,
    gen: RoughGenerator,
}

impl<C: RoughCanvasContext> RoughCanvas<C> {
    pub fn new(ctx: C, config: Option<&Options>) -> Self {
        Self {
            ctx,
            gen: RoughGenerator::new(config),
        }
    }

    pub fn context(&self) -> &C {
        &self.ctx
    }

    pub fn context_mut(&mut self) -> &mut C {
        &mut self.ctx
    }

    pub fn into_context(self) -> C {
        self.ctx
    }

    /// `get generator()`.
    pub fn generator(&mut self) -> &mut RoughGenerator {
        &mut self.gen
    }

    /// `getDefaultOptions()`.
    pub fn default_options(&self) -> &ResolvedOptions {
        self.gen.default_options()
    }

    /// `draw(drawable)`.
    pub fn draw(&mut self, drawable: &Drawable) {
        let o = &drawable.options;
        let ctx = &mut self.ctx;
        let precision = o.fixed_decimal_place_digits;
        for drawing in &drawable.sets {
            match drawing.kind {
                OpSetType::Path => {
                    ctx.save();
                    let stroke = if o.stroke == "none" {
                        "transparent"
                    } else {
                        o.stroke.as_str()
                    };
                    ctx.set_stroke_style(stroke);
                    ctx.set_line_width(o.stroke_width);
                    // An array is truthy even when empty.
                    if let Some(dash) = &o.stroke_line_dash {
                        ctx.set_line_dash(dash);
                    }
                    if let Some(offset) = o.stroke_line_dash_offset.filter(|v| js::truthy(*v)) {
                        ctx.set_line_dash_offset(offset);
                    }
                    draw_to_context(ctx, drawing, precision, CanvasFillRule::NonZero);
                    ctx.restore();
                }
                OpSetType::FillPath => {
                    ctx.save();
                    ctx.set_fill_style(o.fill.as_deref().unwrap_or(""));
                    let rule = if matches!(drawable.shape.as_str(), "curve" | "polygon" | "path") {
                        CanvasFillRule::EvenOdd
                    } else {
                        CanvasFillRule::NonZero
                    };
                    draw_to_context(ctx, drawing, precision, rule);
                    ctx.restore();
                }
                OpSetType::FillSketch => fill_sketch(ctx, drawing, o),
            }
        }
    }

    pub fn line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        options: Option<&Options>,
    ) -> Drawable {
        let d = self.gen.line(x1, y1, x2, y2, options);
        self.draw(&d);
        d
    }

    pub fn rectangle(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<&Options>,
    ) -> Drawable {
        let d = self.gen.rectangle(x, y, width, height, options);
        self.draw(&d);
        d
    }

    pub fn ellipse(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<&Options>,
    ) -> Drawable {
        let d = self.gen.ellipse(x, y, width, height, options);
        self.draw(&d);
        d
    }

    pub fn circle(&mut self, x: f64, y: f64, diameter: f64, options: Option<&Options>) -> Drawable {
        let d = self.gen.circle(x, y, diameter, options);
        self.draw(&d);
        d
    }

    pub fn linear_path(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        let d = self.gen.linear_path(points, options);
        self.draw(&d);
        d
    }

    pub fn polygon(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        let d = self.gen.polygon(points, options);
        self.draw(&d);
        d
    }

    #[allow(clippy::too_many_arguments)]
    pub fn arc(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        start: f64,
        stop: f64,
        closed: bool,
        options: Option<&Options>,
    ) -> Drawable {
        let d = self
            .gen
            .arc(x, y, width, height, start, stop, closed, options);
        self.draw(&d);
        d
    }

    pub fn curve(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        let d = self.gen.curve(points, options);
        self.draw(&d);
        d
    }

    pub fn curves(&mut self, points_list: &[Vec<Point>], options: Option<&Options>) -> Drawable {
        let d = self.gen.curves(points_list, options);
        self.draw(&d);
        d
    }

    /// `path(d)`. The original throws on bad path data before drawing; the
    /// port returns the error and draws nothing.
    pub fn path(&mut self, d: &str, options: Option<&Options>) -> Result<Drawable, PathDataError> {
        let drawing = self.gen.path(d, options)?;
        self.draw(&drawing);
        Ok(drawing)
    }
}

/// `fillSketch(ctx, drawing, o)`.
fn fill_sketch<C: RoughCanvasContext>(ctx: &mut C, drawing: &OpSet, o: &ResolvedOptions) {
    let mut fweight = o.fill_weight;
    if fweight < 0.0 {
        fweight = o.stroke_width / 2.0;
    }
    ctx.save();
    if let Some(dash) = &o.fill_line_dash {
        ctx.set_line_dash(dash);
    }
    if let Some(offset) = o.fill_line_dash_offset.filter(|v| js::truthy(*v)) {
        ctx.set_line_dash_offset(offset);
    }
    ctx.set_stroke_style(o.fill.as_deref().unwrap_or(""));
    ctx.set_line_width(fweight);
    draw_to_context(
        ctx,
        drawing,
        o.fixed_decimal_place_digits,
        CanvasFillRule::NonZero,
    );
    ctx.restore();
}

/// `_drawToContext(ctx, drawing, fixedDecimals, rule)`.
fn draw_to_context<C: RoughCanvasContext>(
    ctx: &mut C,
    drawing: &OpSet,
    fixed_decimals: Option<f64>,
    rule: CanvasFillRule,
) {
    ctx.begin_path();
    for item in &drawing.ops {
        let data: Vec<f64> = match fixed_decimals {
            // `+d.toFixed(fixedDecimals)`
            Some(fd) if fd >= 0.0 => item
                .data
                .iter()
                .map(|d| js::parse_float(&js::to_fixed(*d, fd.trunc() as usize)))
                .collect(),
            _ => item.data.clone(),
        };
        match item.op {
            OpType::Move => ctx.move_to(data[0], data[1]),
            OpType::BCurveTo => {
                ctx.bezier_curve_to(data[0], data[1], data[2], data[3], data[4], data[5])
            }
            OpType::LineTo => ctx.line_to(data[0], data[1]),
        }
    }
    if drawing.kind == OpSetType::FillPath {
        ctx.fill(rule);
    } else {
        ctx.stroke();
    }
}
