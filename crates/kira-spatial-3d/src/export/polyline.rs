use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::Error;
use crate::contour::PolylineSet;
use crate::export::floatfmt::{FloatFmt, fmt_f32, sanitize_f32};
use crate::metrics::RidgeMetrics;

/// TSV float formatting options.
#[derive(Clone, Copy, Debug)]
pub struct TsvOptions {
    pub float: FloatFmt,
}

impl Default for TsvOptions {
    fn default() -> Self {
        Self {
            float: FloatFmt::DEFAULT,
        }
    }
}

#[derive(Serialize)]
pub struct PolylineJson {
    pub level: f32,
    pub polylines: Vec<PolylineJsonItem>,
}

#[derive(Serialize)]
pub struct PolylineJsonItem {
    pub is_closed: bool,
    pub points: Vec<[f32; 3]>,
}

#[derive(Serialize)]
pub struct RidgeMetricsJson {
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

/// Writes compact deterministic JSON for stitched polylines.
pub fn write_polylines_json<W: Write>(set: &PolylineSet, w: W) -> Result<(), Error> {
    let dto = PolylineJson {
        level: sanitize_f32(set.level),
        polylines: set
            .polylines
            .iter()
            .map(|p| PolylineJsonItem {
                is_closed: p.is_closed,
                points: p.points.iter().copied().map(sanitize_point).collect(),
            })
            .collect(),
    };
    serde_json::to_writer(w, &dto)?;
    Ok(())
}

/// Saves compact deterministic JSON for stitched polylines.
pub fn save_polylines_json<P: AsRef<Path>>(set: &PolylineSet, path: P) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_polylines_json(set, writer)
}

/// Writes compact deterministic JSON for ridge metrics.
pub fn write_ridge_metrics_json<W: Write>(m: &RidgeMetrics, w: W) -> Result<(), Error> {
    let dto = RidgeMetricsJson {
        level: sanitize_f32(m.level),
        num_polylines: m.num_polylines,
        num_closed: m.num_closed,
        num_open: m.num_open,
        total_length: sanitize_f32(m.total_length),
        mean_length: sanitize_f32(m.mean_length),
        fragmentation_index: sanitize_f32(m.fragmentation_index),
        num_endpoints: m.num_endpoints,
        mean_abs_turn_angle: sanitize_f32(m.mean_abs_turn_angle),
    };
    serde_json::to_writer(w, &dto)?;
    Ok(())
}

/// Saves compact deterministic JSON for ridge metrics.
pub fn save_ridge_metrics_json<P: AsRef<Path>>(m: &RidgeMetrics, path: P) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_ridge_metrics_json(m, writer)
}

/// Writes one row per polyline point.
pub fn write_polylines_tsv<W: Write>(
    set: &PolylineSet,
    w: W,
    opts: TsvOptions,
) -> Result<(), Error> {
    let mut w = w;
    writeln!(w, "level\tpolyline_id\tpoint_id\tis_closed\tx\ty\tz")?;

    for (polyline_id, p) in set.polylines.iter().enumerate() {
        let is_closed = usize::from(p.is_closed);
        for (point_id, point) in p.points.iter().enumerate() {
            writeln!(
                w,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                fmt_f32(set.level, opts.float),
                polyline_id,
                point_id,
                is_closed,
                fmt_f32(point[0], opts.float),
                fmt_f32(point[1], opts.float),
                fmt_f32(point[2], opts.float),
            )?;
        }
    }
    Ok(())
}

/// Saves one-row-per-point polyline TSV.
pub fn save_polylines_tsv<P: AsRef<Path>>(
    set: &PolylineSet,
    path: P,
    opts: TsvOptions,
) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_polylines_tsv(set, writer, opts)
}

/// Writes single-row ridge metrics TSV with deterministic header order.
pub fn write_ridge_metrics_tsv<W: Write>(
    m: &RidgeMetrics,
    w: W,
    opts: TsvOptions,
) -> Result<(), Error> {
    let mut w = w;
    writeln!(
        w,
        "level\tnum_polylines\tnum_closed\tnum_open\ttotal_length\tmean_length\tfragmentation_index\tnum_endpoints\tmean_abs_turn_angle"
    )?;
    writeln!(
        w,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        fmt_f32(m.level, opts.float),
        m.num_polylines,
        m.num_closed,
        m.num_open,
        fmt_f32(m.total_length, opts.float),
        fmt_f32(m.mean_length, opts.float),
        fmt_f32(m.fragmentation_index, opts.float),
        m.num_endpoints,
        fmt_f32(m.mean_abs_turn_angle, opts.float),
    )?;
    Ok(())
}

/// Saves ridge metrics TSV.
pub fn save_ridge_metrics_tsv<P: AsRef<Path>>(
    m: &RidgeMetrics,
    path: P,
    opts: TsvOptions,
) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_ridge_metrics_tsv(m, writer, opts)
}

#[inline]
fn sanitize_point(p: [f32; 3]) -> [f32; 3] {
    [sanitize_f32(p[0]), sanitize_f32(p[1]), sanitize_f32(p[2])]
}
