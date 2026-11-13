//! A port of `points-on-curve` 0.2.0: flattening Bézier curves,
//! Ramer–Douglas–Peucker simplification, and `curveToBezier`.

use crate::core::Point;
use crate::js;

fn distance(p1: Point, p2: Point) -> f64 {
    distance_sq(p1, p2).sqrt()
}

fn distance_sq(p1: Point, p2: Point) -> f64 {
    js::sq(p1[0] - p2[0]) + js::sq(p1[1] - p2[1])
}

fn distance_to_segment_sq(p: Point, v: Point, w: Point) -> f64 {
    let l2 = distance_sq(v, w);
    if l2 == 0.0 {
        return distance_sq(p, v);
    }
    let mut t = ((p[0] - v[0]) * (w[0] - v[0]) + (p[1] - v[1]) * (w[1] - v[1])) / l2;
    t = js::max(0.0, js::min(1.0, t));
    distance_sq(p, lerp(v, w, t))
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn flatness(points: &[Point], offset: usize) -> f64 {
    let p1 = points[offset];
    let p2 = points[offset + 1];
    let p3 = points[offset + 2];
    let p4 = points[offset + 3];
    let mut ux = 3.0 * p2[0] - 2.0 * p1[0] - p4[0];
    ux *= ux;
    let mut uy = 3.0 * p2[1] - 2.0 * p1[1] - p4[1];
    uy *= uy;
    let mut vx = 3.0 * p3[0] - 2.0 * p4[0] - p1[0];
    vx *= vx;
    let mut vy = 3.0 * p3[1] - 2.0 * p4[1] - p1[1];
    vy *= vy;
    if ux < vx {
        ux = vx;
    }
    if uy < vy {
        uy = vy;
    }
    ux + uy
}

fn points_on_bezier_curve_with_splitting(
    points: &[Point],
    offset: usize,
    tolerance: f64,
    out_points: &mut Vec<Point>,
) {
    if flatness(points, offset) < tolerance {
        let p0 = points[offset];
        if let Some(last) = out_points.last() {
            let d = distance(*last, p0);
            if d > 1.0 {
                out_points.push(p0);
            }
        } else {
            out_points.push(p0);
        }
        out_points.push(points[offset + 3]);
    } else {
        let t = 0.5;
        let p1 = points[offset];
        let p2 = points[offset + 1];
        let p3 = points[offset + 2];
        let p4 = points[offset + 3];
        let q1 = lerp(p1, p2, t);
        let q2 = lerp(p2, p3, t);
        let q3 = lerp(p3, p4, t);
        let r1 = lerp(q1, q2, t);
        let r2 = lerp(q2, q3, t);
        let red = lerp(r1, r2, t);
        points_on_bezier_curve_with_splitting(&[p1, q1, r1, red], 0, tolerance, out_points);
        points_on_bezier_curve_with_splitting(&[red, r2, q3, p4], 0, tolerance, out_points);
    }
}

/// `simplify(points, distance)`.
pub fn simplify(points: &[Point], distance: f64) -> Vec<Point> {
    let mut out = Vec::new();
    simplify_points(points, 0, points.len(), distance, &mut out);
    out
}

fn simplify_points(
    points: &[Point],
    start: usize,
    end: usize,
    epsilon: f64,
    out_points: &mut Vec<Point>,
) {
    // JavaScript reads `points[end - 1]` as undefined for an empty list and
    // then pushes it; an empty input has nothing to simplify here.
    if end == 0 {
        return;
    }
    let s = points[start];
    let e = points[end - 1];
    let mut max_dist_sq = 0.0;
    let mut max_ndx = 1;
    for (i, point) in points.iter().enumerate().take(end - 1).skip(start + 1) {
        let dist_sq = distance_to_segment_sq(*point, s, e);
        if dist_sq > max_dist_sq {
            max_dist_sq = dist_sq;
            max_ndx = i;
        }
    }
    if f64::sqrt(max_dist_sq) > epsilon {
        simplify_points(points, start, max_ndx + 1, epsilon, out_points);
        simplify_points(points, max_ndx, end, epsilon, out_points);
    } else {
        if out_points.is_empty() {
            out_points.push(s);
        }
        out_points.push(e);
    }
}

/// `pointsOnBezierCurves(points, tolerance, distance)`.
pub fn points_on_bezier_curves(
    points: &[Point],
    tolerance: f64,
    distance: Option<f64>,
) -> Vec<Point> {
    let mut new_points = Vec::new();
    // `(points.length - 1) / 3` segments; JavaScript counts a partial one
    // when the division is not exact, and `i < 1.33` still runs i = 1.
    let num_segments = (points.len() as f64 - 1.0) / 3.0;
    let mut i = 0usize;
    while (i as f64) < num_segments {
        points_on_bezier_curve_with_splitting(points, i * 3, tolerance, &mut new_points);
        i += 1;
    }
    if let Some(distance) = distance {
        if distance > 0.0 {
            let mut out = Vec::new();
            simplify_points(&new_points, 0, new_points.len(), distance, &mut out);
            return out;
        }
    }
    new_points
}

/// `curveToBezier(points, curveTightness)`.
pub fn curve_to_bezier(points_in: &[Point], curve_tightness: f64) -> Vec<Point> {
    let len = points_in.len();
    assert!(len >= 3, "A curve must have at least three points.");
    let mut out = Vec::new();
    if len == 3 {
        out.extend([points_in[0], points_in[1], points_in[2], points_in[2]]);
    } else {
        let mut points = vec![points_in[0], points_in[0]];
        for (i, point) in points_in.iter().enumerate().skip(1) {
            points.push(*point);
            if i == len - 1 {
                points.push(*point);
            }
        }
        let s = 1.0 - curve_tightness;
        out.push(points[0]);
        let mut i = 1;
        while i + 2 < points.len() {
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
            out.extend([b1, b2, b3]);
            i += 1;
        }
    }
    out
}
