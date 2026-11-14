//! A port of rough.js's `fillers/`: hachure, zigzag, cross-hatch, dots,
//! dashed and zigzag-line fills.

use std::f64::consts::PI;

use crate::core::{line_length, Line, Op, OpSet, OpSetType, Point, ResolvedOptions};
use crate::hachure_fill::hachure_lines;
use crate::js;
use crate::renderer::{double_line_fill_ops, ellipse};

/// `getFiller(o).fillPolygons(polygonList, o)`.
pub fn fill_polygons(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    match o.fill_style.as_str() {
        "zigzag" => zigzag(polygon_list, o),
        "cross-hatch" => hatch(polygon_list, o),
        "dots" => dots(polygon_list, o),
        "dashed" => dashed(polygon_list, o),
        "zigzag-line" => zigzag_line(polygon_list, o),
        _ => hachure(polygon_list, o),
    }
}

/// `polygonHachureLines` from `scan-line-hachure.ts`.
fn polygon_hachure_lines(polygon_list: &mut [Vec<Point>], o: &ResolvedOptions) -> Vec<Line> {
    let angle = o.hachure_angle + 90.0;
    let mut gap = o.hachure_gap;
    if gap < 0.0 {
        gap = o.stroke_width * 4.0;
    }
    gap = js::round(js::max(gap, 0.1));
    let mut skip_offset = 1.0;
    if o.roughness >= 1.0 {
        // `(o.randomizer?.next() || Math.random()) > 0.7`; this does not
        // create a randomizer when there is none.
        let next = o.randomizer.as_ref().map(|r| r.borrow_mut().next());
        let value = match next {
            Some(n) if js::truthy(n) => n,
            _ => js::math_random(),
        };
        if value > 0.7 {
            skip_offset = gap;
        }
    }
    hachure_lines(polygon_list, gap, angle, js::or(skip_offset, 1.0))
}

fn render_lines(lines: &[Line], o: &mut ResolvedOptions) -> Vec<Op> {
    let mut ops = Vec::new();
    for line in lines {
        ops.extend(double_line_fill_ops(
            line[0][0], line[0][1], line[1][0], line[1][1], o,
        ));
    }
    ops
}

fn hachure(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    let lines = polygon_hachure_lines(polygon_list, o);
    OpSet::new(OpSetType::FillSketch, render_lines(&lines, o))
}

fn zigzag(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    let mut gap = o.hachure_gap;
    if gap < 0.0 {
        gap = o.stroke_width * 4.0;
    }
    gap = js::max(gap, 0.1);
    let mut o2 = o.clone();
    o2.hachure_gap = gap;
    let lines = polygon_hachure_lines(polygon_list, &o2);
    let zig_zag_angle = (PI / 180.0) * o.hachure_angle;
    let mut zigzag_lines = Vec::new();
    let dgx = gap * 0.5 * js::cos(zig_zag_angle);
    let dgy = gap * 0.5 * js::sin(zig_zag_angle);
    for [p1, p2] in &lines {
        if js::truthy(line_length(&[*p1, *p2])) {
            zigzag_lines.push([[p1[0] - dgx, p1[1] + dgy], *p2]);
            zigzag_lines.push([[p1[0] + dgx, p1[1] - dgy], *p2]);
        }
    }
    OpSet::new(OpSetType::FillSketch, render_lines(&zigzag_lines, o))
}

fn hatch(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    let mut set = hachure(polygon_list, o);
    let mut o2 = o.clone();
    o2.hachure_angle = o.hachure_angle + 90.0;
    let set2 = hachure(polygon_list, &mut o2);
    set.ops.extend(set2.ops);
    set
}

fn dots(polygon_list: &mut [Vec<Point>], o: &ResolvedOptions) -> OpSet {
    let mut o = o.clone();
    o.hachure_angle = 0.0;
    let lines = polygon_hachure_lines(polygon_list, &o);
    let mut ops = Vec::new();
    let mut gap = o.hachure_gap;
    if gap < 0.0 {
        gap = o.stroke_width * 4.0;
    }
    gap = js::max(gap, 0.1);
    let mut fweight = o.fill_weight;
    if fweight < 0.0 {
        fweight = o.stroke_width / 2.0;
    }
    let ro = gap / 4.0;
    for line in &lines {
        let length = line_length(line);
        let dl = length / gap;
        let count = dl.ceil() - 1.0;
        let offset = length - (count * gap);
        let x = ((line[0][0] + line[1][0]) / 2.0) - (gap / 4.0);
        let min_y = js::min(line[0][1], line[1][1]);
        let mut i = 0.0;
        while i < count {
            let y = min_y + offset + (i * gap);
            // The original takes these from Math.random even when a seed is set.
            let cx = (x - ro) + js::math_random() * 2.0 * ro;
            let cy = (y - ro) + js::math_random() * 2.0 * ro;
            ops.extend(ellipse(cx, cy, fweight, fweight, &mut o).ops);
            i += 1.0;
        }
    }
    OpSet::new(OpSetType::FillSketch, ops)
}

/// The `p1` and `p2` of a line, left to right, and its angle.
fn left_to_right(line: &Line) -> (Point, Point, f64) {
    let (mut p1, mut p2) = (line[0], line[1]);
    if p1[0] > p2[0] {
        p1 = line[1];
        p2 = line[0];
    }
    let alpha = js::atan((p2[1] - p1[1]) / (p2[0] - p1[0]));
    (p1, p2, alpha)
}

fn dashed(polygon_list: &mut [Vec<Point>], o: &mut ResolvedOptions) -> OpSet {
    let lines = polygon_hachure_lines(polygon_list, o);
    let gap_or_default = if o.hachure_gap < 0.0 {
        o.stroke_width * 4.0
    } else {
        o.hachure_gap
    };
    let offset = if o.dash_offset < 0.0 {
        gap_or_default
    } else {
        o.dash_offset
    };
    let gap = if o.dash_gap < 0.0 {
        gap_or_default
    } else {
        o.dash_gap
    };
    let mut ops = Vec::new();
    for line in &lines {
        let length = line_length(line);
        let count = (length / (offset + gap)).floor();
        let start_offset = (length + gap - (count * (offset + gap))) / 2.0;
        let (p1, _, alpha) = left_to_right(line);
        let mut i = 0.0;
        while i < count {
            let lstart = i * (offset + gap);
            let lend = lstart + offset;
            let start = [
                p1[0] + (lstart * js::cos(alpha)) + (start_offset * js::cos(alpha)),
                p1[1] + lstart * js::sin(alpha) + (start_offset * js::sin(alpha)),
            ];
            let end = [
                p1[0] + (lend * js::cos(alpha)) + (start_offset * js::cos(alpha)),
                p1[1] + (lend * js::sin(alpha)) + (start_offset * js::sin(alpha)),
            ];
            ops.extend(double_line_fill_ops(start[0], start[1], end[0], end[1], o));
            i += 1.0;
        }
    }
    OpSet::new(OpSetType::FillSketch, ops)
}

fn zigzag_line(polygon_list: &mut [Vec<Point>], o: &ResolvedOptions) -> OpSet {
    let gap = if o.hachure_gap < 0.0 {
        o.stroke_width * 4.0
    } else {
        o.hachure_gap
    };
    let zo = if o.zigzag_offset < 0.0 {
        gap
    } else {
        o.zigzag_offset
    };
    let mut o = o.clone();
    o.hachure_gap = gap + zo;
    let lines = polygon_hachure_lines(polygon_list, &o);
    let mut ops = Vec::new();
    for line in &lines {
        let length = line_length(line);
        let count = js::round(length / (2.0 * zo));
        let (p1, _, alpha) = left_to_right(line);
        let mut i = 0.0;
        while i < count {
            let lstart = i * 2.0 * zo;
            let lend = (i + 1.0) * 2.0 * zo;
            let dz = (2.0 * js::sq(zo)).sqrt();
            let start = [
                p1[0] + (lstart * js::cos(alpha)),
                p1[1] + lstart * js::sin(alpha),
            ];
            let end = [
                p1[0] + (lend * js::cos(alpha)),
                p1[1] + (lend * js::sin(alpha)),
            ];
            let middle = [
                start[0] + dz * js::cos(alpha + PI / 4.0),
                start[1] + dz * js::sin(alpha + PI / 4.0),
            ];
            ops.extend(double_line_fill_ops(
                start[0], start[1], middle[0], middle[1], &mut o,
            ));
            ops.extend(double_line_fill_ops(
                middle[0], middle[1], end[0], end[1], &mut o,
            ));
            i += 1.0;
        }
    }
    OpSet::new(OpSetType::FillSketch, ops)
}
