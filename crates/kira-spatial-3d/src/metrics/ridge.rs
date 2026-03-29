use crate::Error;
use crate::contour::{ContourSet, PolylineSet, StitchOptions, stitch_contours};

/// Summary ridge/contour metrics for one iso-level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RidgeMetrics {
    pub level: f32,
    pub num_polylines: usize,
    pub num_closed: usize,
    pub num_open: usize,
    pub total_length: f32,
    pub mean_length: f32,
    pub fragmentation_index: f32,
    pub num_endpoints: usize,
    pub mean_abs_turn_angle: f32,
}

/// Computes deterministic ridge metrics from stitched polylines.
pub fn compute_ridge_metrics(set: &PolylineSet) -> RidgeMetrics {
    let num_polylines = set.polylines.len();
    let num_closed = set.polylines.iter().filter(|p| p.is_closed).count();
    let num_open = num_polylines.saturating_sub(num_closed);
    let num_endpoints = num_open * 2;

    let mut total_length = 0.0_f32;
    let mut turn_sum = 0.0_f32;
    let mut turn_count = 0_usize;

    for poly in &set.polylines {
        total_length += polyline_length(poly);
        accumulate_turn_angles(poly, &mut turn_sum, &mut turn_count);
    }

    let mean_length = if num_polylines > 0 {
        total_length / num_polylines as f32
    } else {
        0.0
    };
    let fragmentation_index = num_polylines as f32 / total_length.max(1.0);
    let mean_abs_turn_angle = if turn_count > 0 {
        turn_sum / turn_count as f32
    } else {
        0.0
    };

    RidgeMetrics {
        level: set.level,
        num_polylines,
        num_closed,
        num_open,
        total_length,
        mean_length,
        fragmentation_index,
        num_endpoints,
        mean_abs_turn_angle,
    }
}

/// Convenience helper: stitch ridge contours and compute ridge metrics.
pub fn ridges_to_polylines_and_metrics(
    contours: &ContourSet,
    stitch: StitchOptions,
) -> Result<(PolylineSet, RidgeMetrics), Error> {
    let polylines = stitch_contours(contours, stitch)?;
    let metrics = compute_ridge_metrics(&polylines);
    Ok((polylines, metrics))
}

fn polyline_length(poly: &crate::contour::Polyline) -> f32 {
    if poly.points.len() < 2 {
        return 0.0;
    }

    let mut len = 0.0_f32;
    for i in 1..poly.points.len() {
        len += dist(poly.points[i - 1], poly.points[i]);
    }
    if poly.is_closed {
        len += dist(poly.points[poly.points.len() - 1], poly.points[0]);
    }
    len
}

fn accumulate_turn_angles(poly: &crate::contour::Polyline, sum: &mut f32, count: &mut usize) {
    let n = poly.points.len();
    if n < 3 {
        return;
    }

    if poly.is_closed {
        for i in 0..n {
            let prev = poly.points[(i + n - 1) % n];
            let cur = poly.points[i];
            let next = poly.points[(i + 1) % n];
            if let Some(angle) = turn_angle(prev, cur, next) {
                *sum += angle;
                *count += 1;
            }
        }
    } else {
        for i in 1..(n - 1) {
            let prev = poly.points[i - 1];
            let cur = poly.points[i];
            let next = poly.points[i + 1];
            if let Some(angle) = turn_angle(prev, cur, next) {
                *sum += angle;
                *count += 1;
            }
        }
    }
}

#[inline]
fn turn_angle(prev: [f32; 3], cur: [f32; 3], next: [f32; 3]) -> Option<f32> {
    let a = [cur[0] - prev[0], cur[1] - prev[1], cur[2] - prev[2]];
    let b = [next[0] - cur[0], next[1] - cur[1], next[2] - cur[2]];

    let la2 = dot(a, a);
    let lb2 = dot(b, b);
    if la2 == 0.0 || lb2 == 0.0 {
        return None;
    }
    let denom = la2.sqrt() * lb2.sqrt();
    if denom == 0.0 {
        return None;
    }

    let c = (dot(a, b) / denom).clamp(-1.0, 1.0);
    Some(c.acos())
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
