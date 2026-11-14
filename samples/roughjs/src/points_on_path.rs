//! A port of `points-on-path` 0.2.1: the point sets an SVG path traces.

use crate::core::Point;
use crate::path_data::{absolutize, normalize, parse_path, PathDataError};
use crate::points_on_curve::{points_on_bezier_curves, simplify};

/// `pointsOnPath(path, tolerance, distance)`.
pub fn points_on_path(
    path: &str,
    tolerance: f64,
    distance: Option<f64>,
) -> Result<Vec<Vec<Point>>, PathDataError> {
    let segments = parse_path(path)?;
    let normalized = normalize(&absolutize(&segments));
    let mut sets: Vec<Vec<Point>> = Vec::new();
    let mut current_points: Vec<Point> = Vec::new();
    let mut start: Point = [0.0, 0.0];
    let mut pending_curve: Vec<Point> = Vec::new();

    fn append_pending_curve(pending: &mut Vec<Point>, current: &mut Vec<Point>, tolerance: f64) {
        if pending.len() >= 4 {
            current.extend(points_on_bezier_curves(pending, tolerance, None));
        }
        pending.clear();
    }
    fn append_pending_points(
        pending: &mut Vec<Point>,
        current: &mut Vec<Point>,
        sets: &mut Vec<Vec<Point>>,
        tolerance: f64,
    ) {
        append_pending_curve(pending, current, tolerance);
        if !current.is_empty() {
            sets.push(std::mem::take(current));
        }
    }

    for segment in &normalized {
        let data = &segment.data;
        match segment.key {
            'M' => {
                append_pending_points(
                    &mut pending_curve,
                    &mut current_points,
                    &mut sets,
                    tolerance,
                );
                start = [data[0], data[1]];
                current_points.push(start);
            }
            'L' => {
                append_pending_curve(&mut pending_curve, &mut current_points, tolerance);
                current_points.push([data[0], data[1]]);
            }
            'C' => {
                if pending_curve.is_empty() {
                    let last_point = current_points.last().copied().unwrap_or(start);
                    pending_curve.push(last_point);
                }
                pending_curve.push([data[0], data[1]]);
                pending_curve.push([data[2], data[3]]);
                pending_curve.push([data[4], data[5]]);
            }
            'Z' => {
                append_pending_curve(&mut pending_curve, &mut current_points, tolerance);
                current_points.push(start);
            }
            _ => {}
        }
    }
    append_pending_points(
        &mut pending_curve,
        &mut current_points,
        &mut sets,
        tolerance,
    );
    let Some(distance) = distance.filter(|d| crate::js::truthy(*d)) else {
        return Ok(sets);
    };
    Ok(sets
        .iter()
        .map(|set| simplify(set, distance))
        .filter(|set| !set.is_empty())
        .collect())
}
