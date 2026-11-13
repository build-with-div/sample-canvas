//! A port of rough.js's `RoughGenerator` (`generator.ts`).

use crate::core::{
    random_seed, Drawable, Op, OpSet, OpSetType, OpType, Options, PathInfo, Point, ResolvedOptions,
};
use crate::js;
use crate::path_data::PathDataError;
use crate::points_on_curve::{curve_to_bezier, points_on_bezier_curves};
use crate::points_on_path::points_on_path;
use crate::renderer;

const NOS: &str = "none";

/// `new RoughGenerator(config)`.
///
/// Like the original, a call without options draws its randomness from
/// the generator's own default options, so such calls continue one random
/// sequence; a call with options starts from a copy of the defaults.
#[derive(Debug, Clone, Default)]
pub struct RoughGenerator {
    default_options: ResolvedOptions,
}

impl RoughGenerator {
    /// `new RoughGenerator({ options })`.
    pub fn new(options: Option<&Options>) -> Self {
        let defaults = ResolvedOptions::default();
        let default_options = match options {
            Some(options) => defaults.merged(options),
            None => defaults,
        };
        Self { default_options }
    }

    /// `RoughGenerator.newSeed()`.
    pub fn new_seed() -> f64 {
        random_seed()
    }

    pub fn default_options(&self) -> &ResolvedOptions {
        &self.default_options
    }

    /// `_o(options)` applied to `f`: without options, `f` works on the
    /// defaults themselves, as the original returns the same object.
    fn with_options<R>(
        &mut self,
        options: Option<&Options>,
        f: impl FnOnce(&mut ResolvedOptions) -> R,
    ) -> R {
        match options {
            None => f(&mut self.default_options),
            Some(options) => {
                let mut o = self.default_options.merged(options);
                f(&mut o)
            }
        }
    }

    fn drawable(shape: &str, sets: Vec<OpSet>, o: &ResolvedOptions) -> Drawable {
        Drawable {
            shape: shape.into(),
            sets,
            options: o.clone(),
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
        self.with_options(options, |o| {
            let set = renderer::line(x1, y1, x2, y2, o);
            Self::drawable("line", vec![set], o)
        })
    }

    pub fn rectangle(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<&Options>,
    ) -> Drawable {
        self.with_options(options, |o| {
            let mut paths = Vec::new();
            let outline = renderer::rectangle(x, y, width, height, o);
            if o.has_fill() {
                let mut polygons = vec![vec![
                    [x, y],
                    [x + width, y],
                    [x + width, y + height],
                    [x, y + height],
                ]];
                if o.fill_style == "solid" {
                    paths.push(renderer::solid_fill_polygon(&polygons, o));
                } else {
                    paths.push(renderer::pattern_fill_polygons(&mut polygons, o));
                }
            }
            if o.stroke != NOS {
                paths.push(outline);
            }
            Self::drawable("rectangle", paths, o)
        })
    }

    pub fn ellipse(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<&Options>,
    ) -> Drawable {
        self.with_options(options, |o| ellipse_drawable(x, y, width, height, o))
    }

    pub fn circle(&mut self, x: f64, y: f64, diameter: f64, options: Option<&Options>) -> Drawable {
        let mut drawable = self.ellipse(x, y, diameter, diameter, options);
        drawable.shape = "circle".into();
        drawable
    }

    pub fn linear_path(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        self.with_options(options, |o| {
            let set = renderer::linear_path(points, false, o);
            Self::drawable("linearPath", vec![set], o)
        })
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
        self.with_options(options, |o| {
            let mut paths = Vec::new();
            let outline = renderer::arc(x, y, width, height, start, stop, closed, true, o);
            if closed && o.has_fill() {
                if o.fill_style == "solid" {
                    let mut fill_options = o.clone();
                    fill_options.disable_multi_stroke = true;
                    let mut shape = renderer::arc(
                        x,
                        y,
                        width,
                        height,
                        start,
                        stop,
                        true,
                        false,
                        &mut fill_options,
                    );
                    shape.kind = OpSetType::FillPath;
                    paths.push(shape);
                } else {
                    paths.push(renderer::pattern_fill_arc(
                        x, y, width, height, start, stop, o,
                    ));
                }
            }
            if o.stroke != NOS {
                paths.push(outline);
            }
            Self::drawable("arc", paths, o)
        })
    }

    /// `curve(points)` for one curve.
    pub fn curve(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        self.curves(&[points.to_vec()], options)
    }

    /// `curve(points)` for several curves (`Point[][]`).
    pub fn curves(&mut self, points_list: &[Vec<Point>], options: Option<&Options>) -> Drawable {
        self.with_options(options, |o| {
            let mut paths = Vec::new();
            let outline = renderer::curve(points_list, o);
            if o.has_fill() && o.fill.as_deref() != Some(NOS) {
                if o.fill_style == "solid" {
                    let mut fill_options = o.clone();
                    fill_options.disable_multi_stroke = true;
                    fill_options.roughness = if js::truthy(o.roughness) {
                        o.roughness + o.fill_shape_roughness_gain
                    } else {
                        0.0
                    };
                    let fill_shape = renderer::curve(points_list, &mut fill_options);
                    paths.push(OpSet::new(
                        OpSetType::FillPath,
                        merged_shape(fill_shape.ops),
                    ));
                } else {
                    let mut poly_points = Vec::new();
                    let distance = (1.0 + o.roughness) / 2.0;
                    for points in points_list {
                        if points.len() < 3 {
                            poly_points.extend_from_slice(points);
                        } else if points.len() == 3 {
                            poly_points.extend(points_on_bezier_curves(
                                &curve_to_bezier(
                                    &[points[0], points[0], points[1], points[2]],
                                    0.0,
                                ),
                                10.0,
                                Some(distance),
                            ));
                        } else {
                            poly_points.extend(points_on_bezier_curves(
                                &curve_to_bezier(points, 0.0),
                                10.0,
                                Some(distance),
                            ));
                        }
                    }
                    if !poly_points.is_empty() {
                        paths.push(renderer::pattern_fill_polygons(&mut [poly_points], o));
                    }
                }
            }
            if o.stroke != NOS {
                paths.push(outline);
            }
            Self::drawable("curve", paths, o)
        })
    }

    pub fn polygon(&mut self, points: &[Point], options: Option<&Options>) -> Drawable {
        self.with_options(options, |o| {
            let mut paths = Vec::new();
            let outline = renderer::linear_path(points, true, o);
            if o.has_fill() {
                let mut polygons = vec![points.to_vec()];
                if o.fill_style == "solid" {
                    paths.push(renderer::solid_fill_polygon(&polygons, o));
                } else {
                    paths.push(renderer::pattern_fill_polygons(&mut polygons, o));
                }
            }
            if o.stroke != NOS {
                paths.push(outline);
            }
            Self::drawable("polygon", paths, o)
        })
    }

    /// `path(d)`. The original throws on bad path data; the port returns
    /// the error.
    pub fn path(&mut self, d: &str, options: Option<&Options>) -> Result<Drawable, PathDataError> {
        self.with_options(options, |o| {
            let mut paths = Vec::new();
            if d.is_empty() {
                return Ok(Self::drawable("path", paths, o));
            }
            let d = clean_path_data(d);
            let has_fill = o.has_fill()
                && o.fill.as_deref() != Some("transparent")
                && o.fill.as_deref() != Some(NOS);
            let has_stroke = o.stroke != NOS;
            let simplified = o.simplification.is_some_and(|s| js::truthy(s) && s < 1.0);
            let distance = if simplified {
                4.0 - 4.0 * js::or(o.simplification.unwrap_or(0.0), 1.0)
            } else {
                (1.0 + o.roughness) / 2.0
            };
            let mut sets = points_on_path(&d, 1.0, Some(distance))?;
            let shape = renderer::svg_path(&d, o)?;
            if has_fill {
                if o.fill_style == "solid" {
                    if sets.len() == 1 {
                        let mut fill_options = o.clone();
                        fill_options.disable_multi_stroke = true;
                        fill_options.roughness = if js::truthy(o.roughness) {
                            o.roughness + o.fill_shape_roughness_gain
                        } else {
                            0.0
                        };
                        let fill_shape = renderer::svg_path(&d, &mut fill_options)?;
                        paths.push(OpSet::new(
                            OpSetType::FillPath,
                            merged_shape(fill_shape.ops),
                        ));
                    } else {
                        paths.push(renderer::solid_fill_polygon(&sets, o));
                    }
                } else {
                    paths.push(renderer::pattern_fill_polygons(&mut sets, o));
                }
            }
            if has_stroke {
                if simplified {
                    for set in &sets {
                        paths.push(renderer::linear_path(set, false, o));
                    }
                } else {
                    paths.push(shape);
                }
            }
            Ok(Self::drawable("path", paths, o))
        })
    }

    /// `opsToPath(drawing, fixedDecimals)`.
    pub fn ops_to_path(&self, drawing: &OpSet, fixed_decimals: Option<f64>) -> String {
        let mut path = String::new();
        for item in &drawing.ops {
            let data: Vec<String> =
                item.data
                    .iter()
                    .map(|d| match fixed_decimals {
                        Some(fd) if fd >= 0.0 => js::number_to_string(js::parse_float(
                            &js::to_fixed(*d, fd.trunc() as usize),
                        )),
                        _ => js::number_to_string(*d),
                    })
                    .collect();
            match item.op {
                OpType::Move => path += &format!("M{} {} ", data[0], data[1]),
                OpType::BCurveTo => {
                    path += &format!(
                        "C{} {}, {} {}, {} {} ",
                        data[0], data[1], data[2], data[3], data[4], data[5]
                    )
                }
                OpType::LineTo => path += &format!("L{} {} ", data[0], data[1]),
            }
        }
        path.trim().to_string()
    }

    /// `toPaths(drawable)`.
    pub fn to_paths(&self, drawable: &Drawable) -> Vec<PathInfo> {
        let o = &drawable.options;
        let fill_or_none = || {
            o.fill
                .clone()
                .filter(|f| !f.is_empty())
                .unwrap_or_else(|| NOS.into())
        };
        drawable
            .sets
            .iter()
            .map(|drawing| match drawing.kind {
                OpSetType::Path => PathInfo {
                    d: self.ops_to_path(drawing, None),
                    stroke: o.stroke.clone(),
                    stroke_width: o.stroke_width,
                    fill: NOS.into(),
                },
                OpSetType::FillPath => PathInfo {
                    d: self.ops_to_path(drawing, None),
                    stroke: NOS.into(),
                    stroke_width: 0.0,
                    fill: fill_or_none(),
                },
                OpSetType::FillSketch => {
                    let mut fweight = o.fill_weight;
                    if fweight < 0.0 {
                        fweight = o.stroke_width / 2.0;
                    }
                    PathInfo {
                        d: self.ops_to_path(drawing, None),
                        stroke: fill_or_none(),
                        stroke_width: fweight,
                        fill: NOS.into(),
                    }
                }
            })
            .collect()
    }
}

fn ellipse_drawable(x: f64, y: f64, width: f64, height: f64, o: &mut ResolvedOptions) -> Drawable {
    let mut paths = Vec::new();
    let params = renderer::generate_ellipse_params(width, height, o);
    let response = renderer::ellipse_with_params(x, y, o, &params);
    if o.has_fill() {
        if o.fill_style == "solid" {
            let mut shape = renderer::ellipse_with_params(x, y, o, &params).opset;
            shape.kind = OpSetType::FillPath;
            paths.push(shape);
        } else {
            let mut polygons = vec![response.estimated_points];
            paths.push(renderer::pattern_fill_polygons(&mut polygons, o));
        }
    }
    if o.stroke != NOS {
        paths.push(response.opset);
    }
    RoughGenerator::drawable("ellipse", paths, o)
}

/// `_mergedShape`: drops every `move` after the first op.
fn merged_shape(input: Vec<Op>) -> Vec<Op> {
    input
        .into_iter()
        .enumerate()
        .filter(|(i, d)| *i == 0 || d.op != OpType::Move)
        .map(|(_, d)| d)
        .collect()
}

/// JavaScript's `\s`.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `d.replace(/\n/g, ' ').replace(/(-\s)/g, '-').replace('/(\s\s)/g', ' ')`.
/// The last call passes a string, not a regular expression, so it replaces
/// that literal text once; the port keeps that.
fn clean_path_data(d: &str) -> String {
    let d = d.replace('\n', " ");
    let mut out = String::with_capacity(d.len());
    let mut chars = d.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '-' && chars.peek().copied().is_some_and(is_js_space) {
            chars.next();
        }
    }
    out.replacen("/(\\s\\s)/g", " ", 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_path_data_like_the_original() {
        assert_eq!(clean_path_data("M0 0\nL- 5 5"), "M0 0 L-5 5");
        // `- ` followed by another space: the first match consumes one space.
        assert_eq!(clean_path_data("L-  5"), "L- 5");
        assert_eq!(clean_path_data("a/(\\s\\s)/gb"), "a b");
    }
}
