//! rough.js's types (`core.ts`, `geometry.ts`, `math.ts`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::js;

/// A point, `[x, y]`.
pub type Point = [f64; 2];
/// A line, `[p1, p2]`.
pub type Line = [Point; 2];

/// `lineLength` from `geometry.ts`.
pub fn line_length(line: &Line) -> f64 {
    let [p1, p2] = line;
    (js::sq(p1[0] - p2[0]) + js::sq(p1[1] - p2[1])).sqrt()
}

/// `Random` from `math.ts`: a Park–Miller generator over `Math.imul`, or
/// `Math.random` when the seed is falsy.
#[derive(Debug, Clone)]
pub struct Random {
    seed: f64,
}

impl Random {
    pub fn new(seed: f64) -> Self {
        Self { seed }
    }

    /// `next()`, named as in rough.js.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        if js::truthy(self.seed) {
            let seed = js::to_int32(self.seed).wrapping_mul(48271);
            self.seed = f64::from(seed);
            f64::from(0x7fff_ffff & seed) / 2_147_483_648.0
        } else {
            js::math_random()
        }
    }
}

/// `randomSeed()`.
pub fn random_seed() -> f64 {
    (js::math_random() * 2_147_483_648.0).floor()
}

/// The options a caller passes; every field is optional, as in rough.js.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Options {
    pub max_randomness_offset: Option<f64>,
    pub roughness: Option<f64>,
    pub bowing: Option<f64>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub curve_fitting: Option<f64>,
    pub curve_tightness: Option<f64>,
    pub curve_step_count: Option<f64>,
    pub fill: Option<String>,
    pub fill_style: Option<String>,
    pub fill_weight: Option<f64>,
    pub hachure_angle: Option<f64>,
    pub hachure_gap: Option<f64>,
    pub simplification: Option<f64>,
    pub dash_offset: Option<f64>,
    pub dash_gap: Option<f64>,
    pub zigzag_offset: Option<f64>,
    pub seed: Option<f64>,
    pub stroke_line_dash: Option<Vec<f64>>,
    pub stroke_line_dash_offset: Option<f64>,
    pub fill_line_dash: Option<Vec<f64>>,
    pub fill_line_dash_offset: Option<f64>,
    pub disable_multi_stroke: Option<bool>,
    pub disable_multi_stroke_fill: Option<bool>,
    pub preserve_vertices: Option<bool>,
    pub fixed_decimal_place_digits: Option<f64>,
    pub fill_shape_roughness_gain: Option<f64>,
}

/// Options with the defaults applied (`ResolvedOptions`).
///
/// `randomizer` is shared between copies, as an object reference is when
/// rough.js copies options with `Object.assign`.
#[derive(Debug, Clone)]
pub struct ResolvedOptions {
    pub max_randomness_offset: f64,
    pub roughness: f64,
    pub bowing: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub curve_fitting: f64,
    pub curve_tightness: f64,
    pub curve_step_count: f64,
    pub fill: Option<String>,
    pub fill_style: String,
    pub fill_weight: f64,
    pub hachure_angle: f64,
    pub hachure_gap: f64,
    pub simplification: Option<f64>,
    pub dash_offset: f64,
    pub dash_gap: f64,
    pub zigzag_offset: f64,
    pub seed: f64,
    pub stroke_line_dash: Option<Vec<f64>>,
    pub stroke_line_dash_offset: Option<f64>,
    pub fill_line_dash: Option<Vec<f64>>,
    pub fill_line_dash_offset: Option<f64>,
    pub disable_multi_stroke: bool,
    pub disable_multi_stroke_fill: bool,
    pub preserve_vertices: bool,
    pub fixed_decimal_place_digits: Option<f64>,
    pub fill_shape_roughness_gain: f64,
    pub randomizer: Option<Rc<RefCell<Random>>>,
}

impl Default for ResolvedOptions {
    /// `RoughGenerator`'s `defaultOptions`.
    fn default() -> Self {
        Self {
            max_randomness_offset: 2.0,
            roughness: 1.0,
            bowing: 1.0,
            stroke: "#000".into(),
            stroke_width: 1.0,
            curve_tightness: 0.0,
            curve_fitting: 0.95,
            curve_step_count: 9.0,
            fill: None,
            fill_style: "hachure".into(),
            fill_weight: -1.0,
            hachure_angle: -41.0,
            hachure_gap: -1.0,
            simplification: None,
            dash_offset: -1.0,
            dash_gap: -1.0,
            zigzag_offset: -1.0,
            seed: 0.0,
            stroke_line_dash: None,
            stroke_line_dash_offset: None,
            fill_line_dash: None,
            fill_line_dash_offset: None,
            disable_multi_stroke: false,
            disable_multi_stroke_fill: false,
            preserve_vertices: false,
            fixed_decimal_place_digits: None,
            fill_shape_roughness_gain: 0.8,
            randomizer: None,
        }
    }
}

impl ResolvedOptions {
    /// `Object.assign({}, this, options)`.
    pub fn merged(&self, options: &Options) -> Self {
        let mut o = self.clone();
        macro_rules! set {
            ($($field:ident),*) => {$(
                if let Some(value) = &options.$field {
                    o.$field = value.clone();
                }
            )*};
        }
        macro_rules! set_optional {
            ($($field:ident),*) => {$(
                if options.$field.is_some() {
                    o.$field = options.$field.clone();
                }
            )*};
        }
        set!(
            max_randomness_offset,
            roughness,
            bowing,
            stroke,
            stroke_width,
            curve_fitting,
            curve_tightness,
            curve_step_count,
            fill_style,
            fill_weight,
            hachure_angle,
            hachure_gap,
            dash_offset,
            dash_gap,
            zigzag_offset,
            seed,
            disable_multi_stroke,
            disable_multi_stroke_fill,
            preserve_vertices,
            fill_shape_roughness_gain
        );
        set_optional!(
            fill,
            simplification,
            stroke_line_dash,
            stroke_line_dash_offset,
            fill_line_dash,
            fill_line_dash_offset,
            fixed_decimal_place_digits
        );
        o
    }

    /// `o.fill` as JavaScript truthiness sees it: set and not empty.
    pub fn has_fill(&self) -> bool {
        self.fill.as_deref().is_some_and(|fill| !fill.is_empty())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    Move,
    BCurveTo,
    LineTo,
}

impl OpType {
    /// The name rough.js uses.
    pub fn as_str(self) -> &'static str {
        match self {
            OpType::Move => "move",
            OpType::BCurveTo => "bcurveTo",
            OpType::LineTo => "lineTo",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Op {
    pub op: OpType,
    pub data: Vec<f64>,
}

impl Op {
    pub(crate) fn move_to(x: f64, y: f64) -> Self {
        Self {
            op: OpType::Move,
            data: vec![x, y],
        }
    }
    pub(crate) fn line_to(x: f64, y: f64) -> Self {
        Self {
            op: OpType::LineTo,
            data: vec![x, y],
        }
    }
    pub(crate) fn bcurve_to(data: [f64; 6]) -> Self {
        Self {
            op: OpType::BCurveTo,
            data: data.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpSetType {
    Path,
    FillPath,
    FillSketch,
}

impl OpSetType {
    /// The name rough.js uses.
    pub fn as_str(self) -> &'static str {
        match self {
            OpSetType::Path => "path",
            OpSetType::FillPath => "fillPath",
            OpSetType::FillSketch => "fillSketch",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpSet {
    pub kind: OpSetType,
    pub ops: Vec<Op>,
}

impl OpSet {
    pub(crate) fn new(kind: OpSetType, ops: Vec<Op>) -> Self {
        Self { kind, ops }
    }
}

#[derive(Debug, Clone)]
pub struct Drawable {
    pub shape: String,
    pub options: ResolvedOptions,
    pub sets: Vec<OpSet>,
}

/// `PathInfo`, from `RoughGenerator::to_paths`.
#[derive(Debug, Clone, PartialEq)]
pub struct PathInfo {
    pub d: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub fill: String,
}
