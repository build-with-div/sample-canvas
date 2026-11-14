//! A port of `hachure-fill` 0.5.2: scan-line hachure lines for polygons.

use std::cmp::Ordering;

use crate::core::{Line, Point};
use crate::js;

fn rotate_points<'a>(points: impl IntoIterator<Item = &'a mut Point>, center: Point, degrees: f64) {
    let [cx, cy] = center;
    let angle = (std::f64::consts::PI / 180.0) * degrees;
    let cos = js::cos(angle);
    let sin = js::sin(angle);
    for p in points {
        let [x, y] = *p;
        p[0] = ((x - cx) * cos) - ((y - cy) * sin) + cx;
        p[1] = ((x - cx) * sin) + ((y - cy) * cos) + cy;
    }
}

/// `hachureLines(polygons, gap, angle, stepOffset)`.
///
/// Like the original, it rotates the caller's polygons in place and back,
/// so their points come back with rounding residue. rough.js's
/// cross-hatch filler depends on that: its second pass sees the moved
/// points.
pub fn hachure_lines(
    polygons: &mut [Vec<Point>],
    hachure_gap: f64,
    hachure_angle: f64,
    hachure_step_offset: f64,
) -> Vec<Line> {
    let angle = hachure_angle;
    let gap = js::max(hachure_gap, 0.1);
    let center = [0.0, 0.0];
    if js::truthy(angle) {
        for polygon in polygons.iter_mut() {
            rotate_points(polygon.iter_mut(), center, angle);
        }
    }
    let mut lines = straight_hachure_lines(polygons, gap, hachure_step_offset);
    if js::truthy(angle) {
        for polygon in polygons.iter_mut() {
            rotate_points(polygon.iter_mut(), center, -angle);
        }
        rotate_points(
            lines.iter_mut().flat_map(|line| line.iter_mut()),
            center,
            -angle,
        );
    }
    lines
}

#[derive(Debug, Clone)]
struct Edge {
    ymin: f64,
    ymax: f64,
    x: f64,
    islope: f64,
}

/// A JavaScript sort comparator's number as an ordering (`NaN` is equal).
fn ordering(n: f64) -> Ordering {
    if n < 0.0 {
        Ordering::Less
    } else if n > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

fn straight_hachure_lines(polygons: &[Vec<Point>], gap: f64, step: f64) -> Vec<Line> {
    let mut vertex_array = Vec::new();
    for polygon in polygons {
        // JavaScript throws reading the first point of an empty polygon;
        // the port skips it.
        let (Some(first), Some(last)) = (polygon.first(), polygon.last()) else {
            continue;
        };
        let mut vertices = polygon.clone();
        if !(first[0] == last[0] && first[1] == last[1]) {
            vertices.push(*first);
        }
        if vertices.len() > 2 {
            vertex_array.push(vertices);
        }
    }
    let mut lines = Vec::new();
    let gap = js::max(gap, 0.1);
    let mut edges = Vec::new();
    for vertices in &vertex_array {
        for pair in vertices.windows(2) {
            let (p1, p2) = (pair[0], pair[1]);
            if p1[1] != p2[1] {
                let ymin = js::min(p1[1], p2[1]);
                edges.push(Edge {
                    ymin,
                    ymax: js::max(p1[1], p2[1]),
                    x: if ymin == p1[1] { p1[0] } else { p2[0] },
                    islope: (p2[0] - p1[0]) / (p2[1] - p1[1]),
                });
            }
        }
    }
    // Both sorts are stable, as V8's is, so equal keys keep their order.
    edges.sort_by(|e1, e2| {
        if e1.ymin < e2.ymin {
            return Ordering::Less;
        }
        if e1.ymin > e2.ymin {
            return Ordering::Greater;
        }
        if e1.x < e2.x {
            return Ordering::Less;
        }
        if e1.x > e2.x {
            return Ordering::Greater;
        }
        if e1.ymax == e2.ymax {
            return Ordering::Equal;
        }
        ordering((e1.ymax - e2.ymax) / (e1.ymax - e2.ymax).abs())
    });
    if edges.is_empty() {
        return lines;
    }
    let mut edges = std::collections::VecDeque::from(edges);
    let mut active: Vec<Edge> = Vec::new();
    let mut y = edges[0].ymin;
    let mut iteration = 0u64;
    while !active.is_empty() || !edges.is_empty() {
        if !edges.is_empty() {
            let take = edges
                .iter()
                .take_while(|edge| edge.ymin.partial_cmp(&y) != Some(Ordering::Greater))
                .count();
            active.extend(edges.drain(..take));
        }
        // `!(ymax <= y)`, which keeps NaN.
        active.retain(|ae| {
            !matches!(
                ae.ymax.partial_cmp(&y),
                Some(Ordering::Less | Ordering::Equal)
            )
        });
        active.sort_by(|a, b| {
            if a.x == b.x {
                return Ordering::Equal;
            }
            ordering((a.x - b.x) / (a.x - b.x).abs())
        });
        if step != 1.0 || (iteration as f64) % gap == 0.0 {
            for pair in active.chunks_exact(2) {
                lines.push([[js::round(pair[0].x), y], [js::round(pair[1].x), y]]);
            }
        }
        y += step;
        for ae in &mut active {
            ae.x += step * ae.islope;
        }
        iteration += 1;
    }
    lines
}
