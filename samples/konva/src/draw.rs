//! A port of Konva's drawing for rects and circles: `drawScene` and
//! `drawHit` of `Shape`, `Container` and `Layer`, the shapes' `sceneFunc`,
//! and the fill, stroke and shadow logic of `SceneContext` and
//! `HitContext` (`Shape.ts`, `Container.ts`, `Layer.ts`, `Context.ts`,
//! `shapes/Rect.ts`, `shapes/Circle.ts`).
//!
//! Each layer draws into its own two canvases, whose contexts record the
//! calls (div-gap: W2). The scene canvas gets what the user sees. The hit
//! canvas gets every shape filled and stroked in its color key; Konva
//! finds the shape under the pointer by reading that canvas's pixel back
//! (W7), which the port can't do yet.
//!
//! Not ported: caching, filters, gradients and patterns, custom
//! `sceneFunc`/`hitFunc`/`clipFunc`, and the isolation canvas (see
//! `use_buffer_canvas`).

use crate::canvas::Canvas;
use crate::color;
use crate::context::{Arg, Context2d};
use crate::js;
use crate::node::{ClassName, KonvaEvent, Node, NodeType, Value};
use std::f64::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Scene,
    Hit,
}

fn arg(v: &Value) -> Arg {
    match v {
        Value::Number(n) => Arg::Number(*n),
        Value::Bool(b) => Arg::Bool(*b),
        Value::String(s) => Arg::Str(s.clone()),
        Value::Array(a) => Arg::Numbers(a.clone()),
    }
}

impl Node {
    /// `draw()`: the scene, then the hit canvas.
    pub fn draw(&self) {
        self.draw_scene(None);
        self.draw_hit(None);
    }

    /// `drawScene(canvas)`: into the given canvas, or the layer's.
    pub fn draw_scene(&self, can: Option<&mut Canvas>) {
        match self.node_type() {
            NodeType::Layer => self.layer_draw(Pass::Scene, can),
            NodeType::Shape => self.shape_draw(Pass::Scene, can),
            _ => self.container_draw(Pass::Scene, can),
        }
    }

    /// `drawHit(canvas)`: into the given canvas, or the layer's hit canvas.
    pub fn draw_hit(&self, can: Option<&mut Canvas>) {
        match self.node_type() {
            NodeType::Layer => self.layer_draw(Pass::Hit, can),
            NodeType::Shape => self.shape_draw(Pass::Hit, can),
            _ => self.container_draw(Pass::Hit, can),
        }
    }

    /// The layer's canvas for a pass, when none is given.
    fn with_canvas(
        &self,
        pass: Pass,
        can: Option<&mut Canvas>,
        f: impl FnOnce(Option<&mut Canvas>),
    ) {
        if can.is_some() {
            return f(can);
        }
        match self.layer() {
            Some(layer) => {
                let mut canvases = layer.0.canvases.borrow_mut();
                f(Some(&mut canvases[if pass == Pass::Scene { 0 } else { 1 }]))
            }
            None => f(None),
        }
    }

    // ---- Layer ----

    fn layer_draw(&self, pass: Pass, can: Option<&mut Canvas>) {
        match pass {
            Pass::Scene => {
                self.fire_own("beforeDraw", &mut KonvaEvent::default());
                self.with_canvas(pass, can, |canvas| {
                    let mut canvas = canvas;
                    if self.flag("clearBeforeDraw", true) {
                        canvas.as_deref_mut().unwrap().clear();
                    }
                    self.container_draw(pass, canvas);
                });
                // `_trimIsolationCanvas`: the port never makes one.
                self.fire_own("draw", &mut KonvaEvent::default());
            }
            Pass::Hit => {
                if can.is_none() {
                    // Allocate or release the hit canvas to match
                    // `listening`, then clear it.
                    self.sync_hit_canvas_size();
                    if self.flag("clearBeforeDraw", true) {
                        self.0.canvases.borrow_mut()[1].clear();
                    }
                }
                self.with_canvas(pass, can, |canvas| self.container_draw(pass, canvas));
            }
        }
    }

    /// `_setSize({width, height})`: a layer's canvases follow the stage.
    pub(crate) fn set_layer_size(&self, width: f64, height: f64) {
        self.0.canvases.borrow_mut()[0].set_size(width, height);
        self.sync_hit_canvas_size();
        self.set_smooth_enabled();
    }

    /// `_syncHitCanvasSize()`: a layer that isn't listening keeps a 0×0
    /// hit canvas.
    fn sync_hit_canvas_size(&self) {
        let listening = self.is_listening();
        let or_zero = |v: f64| if js::truthy(v) { v } else { 0.0 };
        let width = or_zero(if listening { self.width() } else { 0.0 });
        let height = or_zero(if listening { self.height() } else { 0.0 });
        self.0.canvases.borrow_mut()[1].set_size_if_changed(width, height);
    }

    /// `_setSmoothEnabled()`.
    pub(crate) fn set_smooth_enabled(&self) {
        let value = self
            .attr("imageSmoothingEnabled")
            .unwrap_or(Value::Bool(true));
        self.0.canvases.borrow_mut()[0]
            .context
            .set_property("imageSmoothingEnabled", arg(&value));
    }

    /// `Stage._resizeDOM()`: every layer takes the new size and redraws.
    pub(crate) fn resize_dom(&self) {
        let (width, height) = (self.width(), self.height());
        for layer in self.children() {
            layer.set_layer_size(width, height);
            layer.draw();
        }
    }

    // ---- Containers ----

    /// `shouldDrawHit()`: visible and listening; a stage always.
    pub fn should_draw_hit(&self) -> bool {
        self.node_type() == NodeType::Stage || (self.is_listening() && self.is_visible())
    }

    fn container_draw(&self, pass: Pass, can: Option<&mut Canvas>) {
        let draw = match pass {
            Pass::Scene => self.is_visible(),
            Pass::Hit => self.should_draw_hit(),
        };
        if draw {
            self.with_canvas(pass, can, |canvas| self.draw_children(pass, canvas));
        }
    }

    /// `_drawChildren`: under the container's clip and composite
    /// operation.
    fn draw_children(&self, pass: Pass, canvas: Option<&mut Canvas>) {
        let mut canvas = canvas;
        let has_clip = matches!(
            (self.attr("clipWidth"), self.attr("clipHeight")),
            (Some(Value::Number(_)), Some(Value::Number(_)))
        );
        if has_clip {
            let ctx = &mut canvas
                .as_deref_mut()
                .expect("a clip needs a canvas")
                .context;
            ctx.save();
            let transform = self.absolute_transform(None);
            ctx.transform(transform.m);
            ctx.begin_path();
            ctx.rect(
                self.number_or_zero("clipX"),
                self.number_or_zero("clipY"),
                self.number("clipWidth", 0.0),
                self.number("clipHeight", 0.0),
            );
            ctx.clip();
            let mut inverse = transform;
            inverse.invert();
            ctx.transform(inverse.m);
        }
        let composite = self.string("globalCompositeOperation");
        let has_composition =
            pass == Pass::Scene && composite.as_deref().is_some_and(|op| op != "source-over");
        if has_composition {
            let ctx = &mut canvas.as_deref_mut().unwrap().context;
            ctx.save();
            self.apply_composite(ctx);
        }
        if pass == Pass::Scene
            && self.class_name() == ClassName::Group
            && self.flag("isolated", false)
        {
            // An isolated group draws its children into a canvas of its own
            // and composites it as one image.
            panic!(
                "div-gap: W2 W6: an isolated group needs a second canvas drawn with drawImage, \
                 and the context's getTransform; Div has neither"
            );
        }
        for child in self.children() {
            match pass {
                Pass::Scene => child.draw_scene(canvas.as_deref_mut()),
                Pass::Hit => child.draw_hit(canvas.as_deref_mut()),
            }
        }
        if has_composition {
            canvas.as_deref_mut().unwrap().context.restore();
        }
        if has_clip {
            canvas.unwrap().context.restore();
        }
    }

    // ---- Shapes ----

    fn shape_draw(&self, pass: Pass, can: Option<&mut Canvas>) {
        if can.is_none() {
            assert!(
                self.layer().is_some(),
                "Konva throws: a shape draws into its layer's canvas"
            );
        }
        self.with_canvas(pass, can, |canvas| {
            let canvas = canvas.unwrap();
            match pass {
                Pass::Scene => self.shape_draw_scene(canvas),
                Pass::Hit => self.shape_draw_hit(canvas),
            }
        });
    }

    fn shape_draw_scene(&self, canvas: &mut Canvas) {
        let has_shadow = self.has_shadow();
        if !self.is_visible() {
            return;
        }
        let ratio = canvas.pixel_ratio;
        let bitmap = js::truthy(canvas.width) && js::truthy(canvas.height);
        let ctx = &mut canvas.context;
        ctx.save();
        if bitmap && self.use_buffer_canvas() {
            panic!(
                "div-gap: W2 W6: Konva draws this shape through its isolation canvas, which needs \
                 the context's getTransform and drawImage of a canvas; Div has neither"
            );
        }
        self.apply_line_join(ctx);
        self.apply_miter_limit(ctx);
        ctx.transform(self.absolute_transform(None).m);
        let opacity = self.absolute_opacity();
        if opacity != 1.0 {
            ctx.set_property("globalAlpha", Arg::Number(opacity));
        }
        self.apply_composite(ctx);
        if has_shadow {
            self.apply_shadow(ctx, ratio);
        }
        self.scene_func(ctx, Pass::Scene, ratio);
        ctx.restore();
    }

    fn shape_draw_hit(&self, canvas: &mut Canvas) {
        if !self.should_draw_hit() {
            return;
        }
        let ratio = canvas.pixel_ratio;
        let ctx = &mut canvas.context;
        ctx.save();
        self.apply_line_join(ctx);
        self.apply_miter_limit(ctx);
        ctx.transform(self.absolute_transform(None).m);
        self.scene_func(ctx, Pass::Hit, ratio);
        ctx.restore();
    }

    /// `_useBufferCanvas()`: whether Konva draws the shape into a separate
    /// canvas first, so that its fill and stroke take the opacity, or cast
    /// the shadow, as one image.
    ///
    /// div-gap: W2 CanvasRenderingContext2D.getTransform, W6 drawImage of a
    /// canvas. Konva sizes that canvas from the context's transform and
    /// draws it back with `drawImage`. The port reports these shapes and
    /// panics when asked to draw one.
    pub fn use_buffer_canvas(&self) -> bool {
        if !self.attr("perfectDrawEnabled").is_none_or(|v| v.truthy()) {
            return false;
        }
        let has_fill = self.has_fill();
        let has_stroke = self.has_stroke();
        if has_fill && has_stroke && self.absolute_opacity() != 1.0 {
            return true;
        }
        has_fill && has_stroke && self.has_shadow() && self.flag("shadowForStrokeEnabled", true)
    }

    /// `hasFill()`: a fill color, and filling enabled.
    pub fn has_fill(&self) -> bool {
        self.attr("fillEnabled").is_none_or(|v| v.truthy())
            && self.attr("fill").is_some_and(|v| v.truthy())
    }

    /// `hasHitStroke()`.
    pub fn has_hit_stroke(&self) -> bool {
        match self.attr("hitStrokeWidth") {
            None => self.has_stroke(),
            Some(Value::String(s)) if s == "auto" => self.has_stroke(),
            Some(width) => self.flag("strokeEnabled", true) && width.truthy(),
        }
    }

    /// `getShadowRGBA()`: the shadow color with `shadowOpacity` applied,
    /// as an `rgba()` string.
    pub fn shadow_rgba(&self) -> Option<String> {
        if !self.has_shadow() {
            return None;
        }
        let shadow_color = self.string("shadowColor");
        let rgba = color::color_to_rgba(shadow_color.as_deref())?;
        let opacity = self.number("shadowOpacity", 1.0);
        let opacity = if js::truthy(opacity) { opacity } else { 1.0 };
        let s = js::number_to_string;
        Some(format!(
            "rgba({},{},{},{})",
            s(rgba.r),
            s(rgba.g),
            s(rgba.b),
            s(rgba.a * opacity)
        ))
    }

    fn apply_line_join(&self, ctx: &mut dyn Context2d) {
        if let Some(v) = self.attr("lineJoin").filter(|v| v.truthy()) {
            ctx.set_property("lineJoin", arg(&v));
        }
    }

    fn apply_miter_limit(&self, ctx: &mut dyn Context2d) {
        if let Some(v) = self.attr("miterLimit") {
            ctx.set_property("miterLimit", arg(&v));
        }
    }

    fn apply_line_cap(&self, ctx: &mut dyn Context2d) {
        if let Some(v) = self.attr("lineCap").filter(|v| v.truthy()) {
            ctx.set_property("lineCap", arg(&v));
        }
    }

    fn apply_composite(&self, ctx: &mut dyn Context2d) {
        if let Some(op) = self.string("globalCompositeOperation") {
            if !op.is_empty() && op != "source-over" {
                ctx.set_property("globalCompositeOperation", Arg::Str(op));
            }
        }
    }

    /// `SceneContext._applyShadow`: blur and offset scale with the shape.
    fn apply_shadow(&self, ctx: &mut dyn Context2d, ratio: f64) {
        let color = self.shadow_rgba().unwrap_or_else(|| "black".into());
        let blur = self.number("shadowBlur", 0.0);
        let (dx, dy) = (
            self.number("shadowOffsetX", 0.0),
            self.number("shadowOffsetY", 0.0),
        );
        let scale = self.absolute_scale();
        let (scale_x, scale_y) = (scale.x * ratio, scale.y * ratio);
        ctx.set_property("shadowColor", Arg::Str(color));
        ctx.set_property(
            "shadowBlur",
            Arg::Number(blur * js::min(&[scale_x.abs(), scale_y.abs()])),
        );
        ctx.set_property("shadowOffsetX", Arg::Number(dx * scale_x));
        ctx.set_property("shadowOffsetY", Arg::Number(dy * scale_y));
    }

    /// `_sceneFunc(context)` of `Rect` and `Circle`.
    fn scene_func(&self, ctx: &mut dyn Context2d, pass: Pass, ratio: f64) {
        ctx.begin_path();
        match self.class_name() {
            ClassName::Rect => {
                let (width, height) = (self.width(), self.height());
                match self.attr("cornerRadius") {
                    Some(radius) if radius.truthy() => {
                        rounded_rect_path(ctx, width, height, &radius)
                    }
                    _ => ctx.rect(0.0, 0.0, width, height),
                }
            }
            ClassName::Circle => {
                let radius = self.number_or_zero("radius").abs();
                ctx.arc(0.0, 0.0, radius, 0.0, PI * 2.0, false);
            }
            other => unreachable!("{other:?} has no sceneFunc"),
        }
        ctx.close_path();
        self.fill_stroke_shape(ctx, pass, ratio);
    }

    /// `context.fillStrokeShape(shape)`.
    fn fill_stroke_shape(&self, ctx: &mut dyn Context2d, pass: Pass, ratio: f64) {
        if self
            .attr("fillAfterStrokeEnabled")
            .is_some_and(|v| v.truthy())
        {
            self.stroke_shape(ctx, pass, ratio);
            self.fill_shape(ctx, pass);
        } else {
            self.fill_shape(ctx, pass);
            self.stroke_shape(ctx, pass, ratio);
        }
    }

    fn fill_func(&self, ctx: &mut dyn Context2d) {
        let rule = self.attr("fillRule").filter(|v| v.truthy());
        match rule {
            Some(Value::String(rule)) => ctx.fill(Some(&rule)),
            _ => ctx.fill(None),
        }
    }

    fn hit_color(&self) -> Arg {
        Arg::Str(
            self.color_key()
                .expect("Konva warns: a destroyed shape has no color key"),
        )
    }

    fn fill_shape(&self, ctx: &mut dyn Context2d, pass: Pass) {
        if !self.attr("fillEnabled").is_none_or(|v| v.truthy()) {
            return;
        }
        match pass {
            Pass::Scene => {
                // `_getFillStyle`: only a color fill is ported.
                if let Some(fill) = self.attr("fill").filter(|v| v.truthy()) {
                    ctx.set_property("fillStyle", arg(&fill));
                    self.fill_func(ctx);
                }
            }
            Pass::Hit => {
                ctx.save();
                ctx.set_property("fillStyle", self.hit_color());
                self.fill_func(ctx);
                ctx.restore();
            }
        }
    }

    fn stroke_shape(&self, ctx: &mut dyn Context2d, pass: Pass, ratio: f64) {
        let scale_enabled = self.flag("strokeScaleEnabled", true);
        let reset = [ratio, 0.0, 0.0, ratio, 0.0, 0.0];
        match pass {
            Pass::Scene => {
                if !self.has_stroke() {
                    return;
                }
                if !scale_enabled {
                    ctx.save();
                }
                self.apply_line_cap(ctx);
                if let Some(Value::Array(dash)) = self.attr("dash") {
                    if self.attr("dashEnabled").is_none_or(|v| v.truthy()) {
                        ctx.set_line_dash(&dash);
                        ctx.set_property(
                            "lineDashOffset",
                            Arg::Number(self.number("dashOffset", 0.0)),
                        );
                    }
                }
                ctx.set_property("lineWidth", Arg::Number(self.stroke_width()));
                let shadow_color = ctx.shadow_color();
                let shadow_for_stroke = self.flag("shadowForStrokeEnabled", true);
                if !shadow_for_stroke {
                    ctx.set_property("shadowColor", Arg::Str("rgba(0,0,0,0)".into()));
                }
                ctx.set_property("strokeStyle", arg(&self.attr("stroke").unwrap()));
                if !scale_enabled {
                    ctx.set_transform(reset);
                }
                ctx.stroke();
                if !shadow_for_stroke {
                    ctx.set_property("shadowColor", shadow_color);
                }
                if !scale_enabled {
                    ctx.restore();
                }
            }
            Pass::Hit => {
                if !self.has_hit_stroke() {
                    return;
                }
                if !scale_enabled {
                    ctx.save();
                    ctx.set_transform(reset);
                }
                self.apply_line_cap(ctx);
                let width = match self.attr("hitStrokeWidth") {
                    Some(Value::Number(w)) => w,
                    _ => self.stroke_width(),
                };
                ctx.set_property("lineWidth", Arg::Number(width));
                ctx.set_property("strokeStyle", self.hit_color());
                ctx.stroke();
                if !scale_enabled {
                    ctx.restore();
                }
            }
        }
    }
}

/// `Util.drawRoundedRectPath`.
fn rounded_rect_path(ctx: &mut dyn Context2d, width: f64, height: f64, corner: &Value) {
    let x0 = if width < 0.0 { width } else { 0.0 };
    let y0 = if height < 0.0 { height } else { 0.0 };
    let (width, height) = (width.abs(), height.abs());
    // `_cornerRadii`; a negative radius would throw in `arc()`.
    let max = js::min(&[width, height]) / 2.0;
    let clamp = |r: f64| js::min(&[js::max(&[if js::truthy(r) { r } else { 0.0 }, 0.0]), max]);
    let [tl, tr, br, bl] = match corner {
        Value::Number(r) => [clamp(*r); 4],
        Value::Array(a) => [0, 1, 2, 3].map(|i| clamp(a.get(i).copied().unwrap_or(0.0))),
        other => panic!("cornerRadius is {other:?}"),
    };
    ctx.move_to(x0 + tl, y0);
    ctx.line_to(x0 + width - tr, y0);
    ctx.arc(x0 + width - tr, y0 + tr, tr, (PI * 3.0) / 2.0, 0.0, false);
    ctx.line_to(x0 + width, y0 + height - br);
    ctx.arc(x0 + width - br, y0 + height - br, br, 0.0, PI / 2.0, false);
    ctx.line_to(x0 + bl, y0 + height);
    ctx.arc(x0 + bl, y0 + height - bl, bl, PI / 2.0, PI, false);
    ctx.line_to(x0, y0 + tl);
    ctx.arc(x0 + tl, y0 + tl, tl, PI, (PI * 3.0) / 2.0, false);
}
