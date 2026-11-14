//! A port of rough.js's `renderer.ts`: the randomized geometry behind
//! every shape.
//!
//! Each random draw is its own statement, in the order JavaScript
//! evaluates the original expressions, so a seeded shape draws the same
//! sequence of numbers.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use crate::core::{Op, OpSet, OpSetType, Point, Random, ResolvedOptions};
use crate::fillers;
use crate::js;
use crate::path_data::{absolutize, normalize, parse_path, PathDataError};

pub fn line(x1: f64, y1: f64, x2: f64, y2: f64, o: &mut ResolvedOptions) -> OpSet {
    OpSet::new(OpSetType::Path, double_line(x1, y1, x2, y2, o, false))
}

pub fn linear_path(points: &[Point], close: bool, o: &mut ResolvedOptions) -> OpSet {
    let len = points.len();
    if len > 2 {
        let mut ops = Vec::new();
        for i in 0..len - 1 {
            ops.extend(double_line(
                points[i][0],
                points[i][1],
                points[i + 1][0],
                points[i + 1][1],
                o,
                false,
            ));
        }
        if close {
            ops.extend(double_line(
                points[len - 1][0],
                points[len - 1][1],
                points[0][0],
                points[0][1],
                o,
                false,
            ));
        }
        return OpSet::new(OpSetType::Path, ops);
    } else if len == 2 {
        return line(points[0][0], points[0][1], points[1][0], points[1][1], o);
    }
    OpSet::new(OpSetType::Path, Vec::new())
}

pub fn polygon(points: &[Point], o: &mut ResolvedOptions) -> OpSet {
    linear_path(points, true, o)
}

pub fn rectangle(x: f64, y: f64, width: f64, height: f64, o: &mut ResolvedOptions) -> OpSet {
    let points = [
        [x, y],
        [x + width, y],
        [x + width, y + height],
        [x, y + height],
    ];
    polygon(&points, o)
}

/// `curve(inputPoints, o)` with the input as a list of curves; a single
/// curve is a list of one, which the original treats the same way.
pub fn curve(points_list: &[Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    if points_list.is_empty() {
        return OpSet::new(OpSetType::Path, Vec::new());
    }
    let mut o1 = curve_with_offset(&points_list[0], 1.0 * (1.0 + o.roughness * 0.2), o);
    let mut o2 = if o.disable_multi_stroke {
        Vec::new()
    } else {
        curve_with_offset(
            &points_list[0],
            1.5 * (1.0 + o.roughness * 0.22),
            &mut clone_options_alter_seed(o),
        )
    };
    for points in &points_list[1..] {
        if !points.is_empty() {
            let underlay = curve_with_offset(points, 1.0 * (1.0 + o.roughness * 0.2), o);
            let overlay = if o.disable_multi_stroke {
                Vec::new()
            } else {
                curve_with_offset(
                    points,
                    1.5 * (1.0 + o.roughness * 0.22),
                    &mut clone_options_alter_seed(o),
                )
            };
            o1.extend(
                underlay
                    .into_iter()
                    .filter(|item| item.op != crate::core::OpType::Move),
            );
            o2.extend(
                overlay
                    .into_iter()
                    .filter(|item| item.op != crate::core::OpType::Move),
            );
        }
    }
    o1.extend(o2);
    OpSet::new(OpSetType::Path, o1)
}

pub fn ellipse(x: f64, y: f64, width: f64, height: f64, o: &mut ResolvedOptions) -> OpSet {
    let params = generate_ellipse_params(width, height, o);
    ellipse_with_params(x, y, o, &params).opset
}

#[derive(Debug, Clone, Copy)]
pub struct EllipseParams {
    pub increment: f64,
    pub rx: f64,
    pub ry: f64,
}

pub struct EllipseResult {
    pub estimated_points: Vec<Point>,
    pub opset: OpSet,
}

pub fn generate_ellipse_params(width: f64, height: f64, o: &mut ResolvedOptions) -> EllipseParams {
    let psq = (PI * 2.0 * ((js::sq(width / 2.0) + js::sq(height / 2.0)) / 2.0).sqrt()).sqrt();
    let step_count = js::max(
        o.curve_step_count,
        (o.curve_step_count / 200f64.sqrt()) * psq,
    )
    .ceil();
    let increment = (PI * 2.0) / step_count;
    let mut rx = (width / 2.0).abs();
    let mut ry = (height / 2.0).abs();
    let curve_fit_randomness = 1.0 - o.curve_fitting;
    rx += offset_opt(rx * curve_fit_randomness, o, 1.0);
    ry += offset_opt(ry * curve_fit_randomness, o, 1.0);
    EllipseParams { increment, rx, ry }
}

pub fn ellipse_with_params(
    x: f64,
    y: f64,
    o: &mut ResolvedOptions,
    params: &EllipseParams,
) -> EllipseResult {
    let inner = offset(0.4, 1.0, o, 1.0);
    let overlap = params.increment * offset(0.1, inner, o, 1.0);
    let (ap1, cp1) = compute_ellipse_points(
        params.increment,
        x,
        y,
        params.rx,
        params.ry,
        1.0,
        overlap,
        o,
    );
    let mut o1 = curve_ops(&ap1, o);
    if !o.disable_multi_stroke && o.roughness != 0.0 {
        let (ap2, _) =
            compute_ellipse_points(params.increment, x, y, params.rx, params.ry, 1.5, 0.0, o);
        o1.extend(curve_ops(&ap2, o));
    }
    EllipseResult {
        estimated_points: cp1,
        opset: OpSet::new(OpSetType::Path, o1),
    }
}

/// The start and stop angles after rough.js's normalization.
fn arc_angles(start: f64, stop: f64) -> (f64, f64) {
    let mut strt = start;
    let mut stp = stop;
    while strt < 0.0 {
        strt += PI * 2.0;
        stp += PI * 2.0;
    }
    if (stp - strt) > (PI * 2.0) {
        strt = 0.0;
        stp = PI * 2.0;
    }
    (strt, stp)
}

#[allow(clippy::too_many_arguments)]
pub fn arc(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start: f64,
    stop: f64,
    closed: bool,
    rough_closure: bool,
    o: &mut ResolvedOptions,
) -> OpSet {
    let cx = x;
    let cy = y;
    let mut rx = (width / 2.0).abs();
    let mut ry = (height / 2.0).abs();
    rx += offset_opt(rx * 0.01, o, 1.0);
    ry += offset_opt(ry * 0.01, o, 1.0);
    let (strt, stp) = arc_angles(start, stop);
    let ellipse_inc = (PI * 2.0) / o.curve_step_count;
    let arc_inc = js::min(ellipse_inc / 2.0, (stp - strt) / 2.0);
    let mut ops = arc_ops(arc_inc, cx, cy, rx, ry, strt, stp, 1.0, o);
    if !o.disable_multi_stroke {
        ops.extend(arc_ops(arc_inc, cx, cy, rx, ry, strt, stp, 1.5, o));
    }
    if closed {
        if rough_closure {
            ops.extend(double_line(
                cx,
                cy,
                cx + rx * js::cos(strt),
                cy + ry * js::sin(strt),
                o,
                false,
            ));
            ops.extend(double_line(
                cx,
                cy,
                cx + rx * js::cos(stp),
                cy + ry * js::sin(stp),
                o,
                false,
            ));
        } else {
            ops.push(Op::line_to(cx, cy));
            ops.push(Op::line_to(
                cx + rx * js::cos(strt),
                cy + ry * js::sin(strt),
            ));
        }
    }
    OpSet::new(OpSetType::Path, ops)
}

pub fn svg_path(path: &str, o: &mut ResolvedOptions) -> Result<OpSet, PathDataError> {
    let segments = normalize(&absolutize(&parse_path(path)?));
    let mut ops = Vec::new();
    let mut first = [0.0, 0.0];
    let mut current = [0.0, 0.0];
    for segment in &segments {
        let data = &segment.data;
        match segment.key {
            'M' => {
                current = [data[0], data[1]];
                first = [data[0], data[1]];
            }
            'L' => {
                ops.extend(double_line(
                    current[0], current[1], data[0], data[1], o, false,
                ));
                current = [data[0], data[1]];
            }
            'C' => {
                let (x1, y1, x2, y2, x, y) = (data[0], data[1], data[2], data[3], data[4], data[5]);
                ops.extend(bezier_to(x1, y1, x2, y2, x, y, current, o));
                current = [x, y];
            }
            'Z' => {
                ops.extend(double_line(
                    current[0], current[1], first[0], first[1], o, false,
                ));
                current = [first[0], first[1]];
            }
            _ => {}
        }
    }
    Ok(OpSet::new(OpSetType::Path, ops))
}

pub fn solid_fill_polygon(polygon_list: &[Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    let mut ops = Vec::new();
    for points in polygon_list {
        if !points.is_empty() {
            let offset = js::or(o.max_randomness_offset, 0.0);
            let len = points.len();
            if len > 2 {
                let x = points[0][0] + offset_opt(offset, o, 1.0);
                let y = points[0][1] + offset_opt(offset, o, 1.0);
                ops.push(Op::move_to(x, y));
                for point in &points[1..] {
                    let x = point[0] + offset_opt(offset, o, 1.0);
                    let y = point[1] + offset_opt(offset, o, 1.0);
                    ops.push(Op::line_to(x, y));
                }
            }
        }
    }
    OpSet::new(OpSetType::FillPath, ops)
}

/// `patternFillPolygons`. The polygons are the caller's and may be moved
/// slightly by the hachure rotation, as in the original.
pub fn pattern_fill_polygons(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    fillers::fill_polygons(polygon_list, o)
}

#[allow(clippy::too_many_arguments)]
pub fn pattern_fill_arc(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start: f64,
    stop: f64,
    o: &mut ResolvedOptions,
) -> OpSet {
    let cx = x;
    let cy = y;
    let mut rx = (width / 2.0).abs();
    let mut ry = (height / 2.0).abs();
    rx += offset_opt(rx * 0.01, o, 1.0);
    ry += offset_opt(ry * 0.01, o, 1.0);
    let (strt, stp) = arc_angles(start, stop);
    let increment = (stp - strt) / o.curve_step_count;
    let mut points = Vec::new();
    let mut angle = strt;
    while angle <= stp {
        points.push([cx + rx * js::cos(angle), cy + ry * js::sin(angle)]);
        angle += increment;
    }
    points.push([cx + rx * js::cos(stp), cy + ry * js::sin(stp)]);
    points.push([cx, cy]);
    pattern_fill_polygons(&mut [points], o)
}

/// `randOffset`, for fillers.
pub fn rand_offset(x: f64, o: &mut ResolvedOptions) -> f64 {
    offset_opt(x, o, 1.0)
}

/// `randOffsetWithRange`, for fillers.
pub fn rand_offset_with_range(min: f64, max: f64, o: &mut ResolvedOptions) -> f64 {
    offset(min, max, o, 1.0)
}

/// `doubleLineFillOps`, for fillers.
pub fn double_line_fill_ops(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    o: &mut ResolvedOptions,
) -> Vec<Op> {
    double_line(x1, y1, x2, y2, o, true)
}

fn clone_options_alter_seed(o: &ResolvedOptions) -> ResolvedOptions {
    let mut result = o.clone();
    result.randomizer = None;
    if js::truthy(o.seed) {
        result.seed = o.seed + 1.0;
    }
    result
}

fn random(o: &mut ResolvedOptions) -> f64 {
    let seed = js::or(o.seed, 0.0);
    let randomizer = o
        .randomizer
        .get_or_insert_with(|| Rc::new(RefCell::new(Random::new(seed))));
    let next = randomizer.borrow_mut().next();
    next
}

fn offset(min: f64, max: f64, o: &mut ResolvedOptions, roughness_gain: f64) -> f64 {
    let scale = o.roughness * roughness_gain;
    scale * ((random(o) * (max - min)) + min)
}

fn offset_opt(x: f64, o: &mut ResolvedOptions, roughness_gain: f64) -> f64 {
    offset(-x, x, o, roughness_gain)
}

fn double_line(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    o: &mut ResolvedOptions,
    filling: bool,
) -> Vec<Op> {
    let single_stroke = if filling {
        o.disable_multi_stroke_fill
    } else {
        o.disable_multi_stroke
    };
    let mut o1 = line_ops(x1, y1, x2, y2, o, true, false);
    if single_stroke {
        return o1;
    }
    o1.extend(line_ops(x1, y1, x2, y2, o, true, true));
    o1
}

#[allow(clippy::too_many_arguments)]
fn line_ops(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    o: &mut ResolvedOptions,
    mv: bool,
    overlay: bool,
) -> Vec<Op> {
    let length_sq = js::sq(x1 - x2) + js::sq(y1 - y2);
    let length = length_sq.sqrt();
    let roughness_gain = if length < 200.0 {
        1.0
    } else if length > 500.0 {
        0.4
    } else {
        (-0.0016668) * length + 1.233334
    };
    let mut offset = js::or(o.max_randomness_offset, 0.0);
    if (offset * offset * 100.0) > length_sq {
        offset = length / 10.0;
    }
    let half_offset = offset / 2.0;
    let diverge_point = 0.2 + random(o) * 0.2;
    let mut mid_disp_x = o.bowing * o.max_randomness_offset * (y2 - y1) / 200.0;
    let mut mid_disp_y = o.bowing * o.max_randomness_offset * (x1 - x2) / 200.0;
    mid_disp_x = offset_opt(mid_disp_x, o, roughness_gain);
    mid_disp_y = offset_opt(mid_disp_y, o, roughness_gain);
    let preserve_vertices = o.preserve_vertices;
    // randomHalf() and randomFull().
    let random_part = |o: &mut ResolvedOptions| {
        offset_opt(
            if overlay { half_offset } else { offset },
            o,
            roughness_gain,
        )
    };
    let mut ops = Vec::new();
    if mv {
        let dx = if preserve_vertices {
            0.0
        } else {
            random_part(o)
        };
        let dy = if preserve_vertices {
            0.0
        } else {
            random_part(o)
        };
        ops.push(Op::move_to(x1 + dx, y1 + dy));
    }
    let r0 = random_part(o);
    let c0 = mid_disp_x + x1 + (x2 - x1) * diverge_point + r0;
    let r1 = random_part(o);
    let c1 = mid_disp_y + y1 + (y2 - y1) * diverge_point + r1;
    let r2 = random_part(o);
    let c2 = mid_disp_x + x1 + 2.0 * (x2 - x1) * diverge_point + r2;
    let r3 = random_part(o);
    let c3 = mid_disp_y + y1 + 2.0 * (y2 - y1) * diverge_point + r3;
    let dx = if preserve_vertices {
        0.0
    } else {
        random_part(o)
    };
    let dy = if preserve_vertices {
        0.0
    } else {
        random_part(o)
    };
    ops.push(Op::bcurve_to([c0, c1, c2, c3, x2 + dx, y2 + dy]));
    ops
}

fn curve_with_offset(points: &[Point], offset: f64, o: &mut ResolvedOptions) -> Vec<Op> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut ps = Vec::new();
    let jitter = |p: Point, o: &mut ResolvedOptions| {
        let x = p[0] + offset_opt(offset, o, 1.0);
        let y = p[1] + offset_opt(offset, o, 1.0);
        [x, y]
    };
    ps.push(jitter(points[0], o));
    ps.push(jitter(points[0], o));
    for i in 1..points.len() {
        ps.push(jitter(points[i], o));
        if i == points.len() - 1 {
            ps.push(jitter(points[i], o));
        }
    }
    curve_ops(&ps, o)
}

/// `_curve(points, null, o)`; rough.js never passes a close point.
fn curve_ops(points: &[Point], o: &mut ResolvedOptions) -> Vec<Op> {
    let len = points.len();
    let mut ops = Vec::new();
    if len > 3 {
        let s = 1.0 - o.curve_tightness;
        ops.push(Op::move_to(points[1][0], points[1][1]));
        let mut i = 1;
        while i + 2 < len {
            let cached = points[i];
            let b1 = [
                cached[0] + (s * points[i + 1][0] - s * points[i - 1][0]) / 6.0,
                cached[1] + (s * points[i + 1][1] - s * points[i - 1][1]) / 6.0,
            ];
            let b2 = [
                points[i + 1][0] + (s * points[i][0] - s * points[i + 2][0]) / 6.0,
                points[i + 1][1] + (s * points[i][1] - s * points[i + 2][1]) / 6.0,
            ];
            let b3 = [points[i + 1][0], points[i + 1][1]];
            ops.push(Op::bcurve_to([b1[0], b1[1], b2[0], b2[1], b3[0], b3[1]]));
            i += 1;
        }
    } else if len == 3 {
        ops.push(Op::move_to(points[1][0], points[1][1]));
        ops.push(Op::bcurve_to([
            points[1][0],
            points[1][1],
            points[2][0],
            points[2][1],
            points[2][0],
            points[2][1],
        ]));
    } else if len == 2 {
        ops.extend(line_ops(
            points[0][0],
            points[0][1],
            points[1][0],
            points[1][1],
            o,
            true,
            true,
        ));
    }
    ops
}

#[allow(clippy::too_many_arguments)]
fn compute_ellipse_points(
    mut increment: f64,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    offset: f64,
    overlap: f64,
    o: &mut ResolvedOptions,
) -> (Vec<Point>, Vec<Point>) {
    let core_only = o.roughness == 0.0;
    let mut core_points = Vec::new();
    let mut all_points = Vec::new();
    if core_only {
        increment /= 4.0;
        all_points.push([cx + rx * js::cos(-increment), cy + ry * js::sin(-increment)]);
        let mut angle = 0.0;
        while angle <= PI * 2.0 {
            let p = [cx + rx * js::cos(angle), cy + ry * js::sin(angle)];
            core_points.push(p);
            all_points.push(p);
            angle += increment;
        }
        all_points.push([cx + rx * js::cos(0.0), cy + ry * js::sin(0.0)]);
        all_points.push([cx + rx * js::cos(increment), cy + ry * js::sin(increment)]);
    } else {
        let rad_offset = offset_opt(0.5, o, 1.0) - (PI / 2.0);
        let point = |scale: f64, angle: f64, o: &mut ResolvedOptions| {
            let x = offset_opt(offset, o, 1.0) + cx + scale * rx * js::cos(angle);
            let y = offset_opt(offset, o, 1.0) + cy + scale * ry * js::sin(angle);
            [x, y]
        };
        all_points.push(point(0.9, rad_offset - increment, o));
        let end_angle = PI * 2.0 + rad_offset - 0.01;
        let mut angle = rad_offset;
        while angle < end_angle {
            let x = offset_opt(offset, o, 1.0) + cx + rx * js::cos(angle);
            let y = offset_opt(offset, o, 1.0) + cy + ry * js::sin(angle);
            core_points.push([x, y]);
            all_points.push([x, y]);
            angle += increment;
        }
        let x =
            offset_opt(offset, o, 1.0) + cx + rx * js::cos(rad_offset + PI * 2.0 + overlap * 0.5);
        let y =
            offset_opt(offset, o, 1.0) + cy + ry * js::sin(rad_offset + PI * 2.0 + overlap * 0.5);
        all_points.push([x, y]);
        all_points.push(point(0.98, rad_offset + overlap, o));
        all_points.push(point(0.9, rad_offset + overlap * 0.5, o));
    }
    (all_points, core_points)
}

#[allow(clippy::too_many_arguments)]
fn arc_ops(
    increment: f64,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    strt: f64,
    stp: f64,
    offset: f64,
    o: &mut ResolvedOptions,
) -> Vec<Op> {
    let rad_offset = strt + offset_opt(0.1, o, 1.0);
    let mut points = Vec::new();
    let x = offset_opt(offset, o, 1.0) + cx + 0.9 * rx * js::cos(rad_offset - increment);
    let y = offset_opt(offset, o, 1.0) + cy + 0.9 * ry * js::sin(rad_offset - increment);
    points.push([x, y]);
    let mut angle = rad_offset;
    while angle <= stp {
        let x = offset_opt(offset, o, 1.0) + cx + rx * js::cos(angle);
        let y = offset_opt(offset, o, 1.0) + cy + ry * js::sin(angle);
        points.push([x, y]);
        angle += increment;
    }
    points.push([cx + rx * js::cos(stp), cy + ry * js::sin(stp)]);
    points.push([cx + rx * js::cos(stp), cy + ry * js::sin(stp)]);
    curve_ops(&points, o)
}

#[allow(clippy::too_many_arguments)]
fn bezier_to(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x: f64,
    y: f64,
    current: Point,
    o: &mut ResolvedOptions,
) -> Vec<Op> {
    let mut ops = Vec::new();
    let ros = [
        js::or(o.max_randomness_offset, 1.0),
        js::or(o.max_randomness_offset, 1.0) + 0.3,
    ];
    let iterations = if o.disable_multi_stroke { 1 } else { 2 };
    let preserve_vertices = o.preserve_vertices;
    for (i, ro) in ros.iter().copied().enumerate().take(iterations) {
        if i == 0 {
            ops.push(Op::move_to(current[0], current[1]));
        } else {
            let dx = if preserve_vertices {
                0.0
            } else {
                offset_opt(ros[0], o, 1.0)
            };
            let dy = if preserve_vertices {
                0.0
            } else {
                offset_opt(ros[0], o, 1.0)
            };
            ops.push(Op::move_to(current[0] + dx, current[1] + dy));
        }
        let f = if preserve_vertices {
            [x, y]
        } else {
            let fx = x + offset_opt(ro, o, 1.0);
            let fy = y + offset_opt(ro, o, 1.0);
            [fx, fy]
        };
        let c0 = x1 + offset_opt(ro, o, 1.0);
        let c1 = y1 + offset_opt(ro, o, 1.0);
        let c2 = x2 + offset_opt(ro, o, 1.0);
        let c3 = y2 + offset_opt(ro, o, 1.0);
        ops.push(Op::bcurve_to([c0, c1, c2, c3, f[0], f[1]]));
    }
    ops
}
