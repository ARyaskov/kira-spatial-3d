use core::fmt;

use crate::Error;
use crate::contour::{ContourSet, PolylineSet, StitchOptions, stitch_contours};

/// Summary ridge/contour metrics for one iso-level.
/// `fragmentation_index = num_polylines / total_length.max(1.0)` — check `total_length == 0` for "undefined".
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

impl fmt::Display for RidgeMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RidgeMetrics {{ level={:.4}, polylines={} (open={}, closed={}), \
             total_length={:.4}, mean_length={:.4}, fragmentation={:.4}, \
             endpoints={}, mean_abs_turn={:.4} }}",
            self.level,
            self.num_polylines,
            self.num_open,
            self.num_closed,
            self.total_length,
            self.mean_length,
            self.fragmentation_index,
            self.num_endpoints,
            self.mean_abs_turn_angle,
        )
    }
}

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

    // Kahan-compensated f64 sum to avoid f32 drift past ~10⁴ segments.
    let mut sum = 0.0_f64;
    let mut c = 0.0_f64;
    let mut add = |x: f64| {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    };
    for i in 1..poly.points.len() {
        add(dist2(poly.points[i - 1], poly.points[i]) as f64);
    }
    if poly.is_closed {
        add(dist2(poly.points[poly.points.len() - 1], poly.points[0]) as f64);
    }
    sum as f32
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
            if let Some(angle) = turn_angle_2d(prev, cur, next) {
                *sum += angle;
                *count += 1;
            }
        }
    } else {
        for i in 1..(n - 1) {
            let prev = poly.points[i - 1];
            let cur = poly.points[i];
            let next = poly.points[i + 1];
            if let Some(angle) = turn_angle_2d(prev, cur, next) {
                *sum += angle;
                *count += 1;
            }
        }
    }
}

#[inline]
fn turn_angle_2d(prev: [f32; 2], cur: [f32; 2], next: [f32; 2]) -> Option<f32> {
    let a = [cur[0] - prev[0], cur[1] - prev[1]];
    let b = [next[0] - cur[0], next[1] - cur[1]];

    let la2 = a[0] * a[0] + a[1] * a[1];
    let lb2 = b[0] * b[0] + b[1] * b[1];
    if la2 == 0.0 || lb2 == 0.0 {
        return None;
    }

    // atan2(|cross|, dot) — stable near collinear/antiparallel where acos collapses.
    let cross = a[0] * b[1] - a[1] * b[0];
    let dotp = a[0] * b[0] + a[1] * b[1];
    if !cross.is_finite() || !dotp.is_finite() {
        return None;
    }
    Some(cross.abs().atan2(dotp))
}

#[inline]
fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}
