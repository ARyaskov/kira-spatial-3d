use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::contour::Quantize;
use crate::export::floatfmt::{FloatFmt, fmt_f32, sanitize_f32};
use crate::mapping::{HeightMapSpec, HeightMode, Normalization};
use crate::{Error, SpatialDomain};

#[derive(Serialize, Clone)]
pub struct Spatial3dMetadata {
    pub version: &'static str,
    pub domain: DomainMeta,
    pub height: HeightMeta,
    pub contour: Option<ContourMeta>,
}

#[derive(Serialize, Clone)]
pub struct DomainMeta {
    pub nx: usize,
    pub ny: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub step_x: f32,
    pub step_y: f32,
}

#[derive(Serialize, Clone)]
pub struct HeightMeta {
    pub mode: String,
    pub normalization: String,
    pub z_scale: f32,
    pub z_offset: f32,
}

#[derive(Serialize, Clone)]
pub struct ContourMeta {
    pub levels: Vec<f32>,
    pub quantize_grid: f32,
}

/// Contour constructor input for metadata generation.
pub struct ContourMetaInput<'a> {
    pub levels: &'a [f32],
    pub quantize: Quantize,
}

impl Spatial3dMetadata {
    /// Creates deterministic metadata from projection/mapping specs.
    pub fn from_specs(
        domain: SpatialDomain,
        height: HeightMapSpec,
        contour: Option<ContourMetaInput<'_>>,
    ) -> Self {
        let contour = contour.map(|c| ContourMeta {
            levels: c.levels.iter().copied().map(sanitize_f32).collect(),
            quantize_grid: sanitize_f32(c.quantize.grid),
        });

        Self {
            version: "kira-spatial-3d/v1",
            domain: DomainMeta {
                nx: domain.nx,
                ny: domain.ny,
                origin_x: sanitize_f32(domain.origin_x),
                origin_y: sanitize_f32(domain.origin_y),
                step_x: sanitize_f32(domain.step_x),
                step_y: sanitize_f32(domain.step_y),
            },
            height: HeightMeta {
                mode: height_mode_name(height.mode).to_string(),
                normalization: normalization_name(height.normalization),
                z_scale: sanitize_f32(height.z_scale),
                z_offset: sanitize_f32(height.z_offset),
            },
            contour,
        }
    }
}

/// Writes compact deterministic metadata JSON.
pub fn write_metadata_json<W: Write>(m: &Spatial3dMetadata, w: W) -> Result<(), Error> {
    let sanitized = sanitize_metadata(m);
    serde_json::to_writer(w, &sanitized)?;
    Ok(())
}

/// Saves compact deterministic metadata JSON.
pub fn save_metadata_json<P: AsRef<Path>>(m: &Spatial3dMetadata, path: P) -> Result<(), Error> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_metadata_json(m, writer)
}

fn sanitize_metadata(m: &Spatial3dMetadata) -> Spatial3dMetadata {
    Spatial3dMetadata {
        version: m.version,
        domain: DomainMeta {
            nx: m.domain.nx,
            ny: m.domain.ny,
            origin_x: sanitize_f32(m.domain.origin_x),
            origin_y: sanitize_f32(m.domain.origin_y),
            step_x: sanitize_f32(m.domain.step_x),
            step_y: sanitize_f32(m.domain.step_y),
        },
        height: HeightMeta {
            mode: m.height.mode.clone(),
            normalization: m.height.normalization.clone(),
            z_scale: sanitize_f32(m.height.z_scale),
            z_offset: sanitize_f32(m.height.z_offset),
        },
        contour: m.contour.as_ref().map(|c| ContourMeta {
            levels: c.levels.iter().copied().map(sanitize_f32).collect(),
            quantize_grid: sanitize_f32(c.quantize_grid),
        }),
    }
}

fn height_mode_name(mode: HeightMode) -> &'static str {
    match mode {
        HeightMode::Raw => "raw",
        HeightMode::Abs => "abs",
        HeightMode::Signed => "signed",
    }
}

fn normalization_name(norm: Normalization) -> String {
    match norm {
        Normalization::None => "none".to_string(),
        Normalization::MinMax { clip } => match clip {
            Some((lo, hi)) => format!(
                "minmax(clip={}, {})",
                fmt_f32(lo, FloatFmt::DEFAULT),
                fmt_f32(hi, FloatFmt::DEFAULT)
            ),
            None => "minmax".to_string(),
        },
        Normalization::RobustZ { clip_z } => match clip_z {
            Some((lo, hi)) => format!(
                "robust_z(clip_z={}, {})",
                fmt_f32(lo, FloatFmt::DEFAULT),
                fmt_f32(hi, FloatFmt::DEFAULT)
            ),
            None => "robust_z".to_string(),
        },
        Normalization::Percentile { lo, hi } => format!(
            "percentile(lo={}, hi={})",
            fmt_f32(lo, FloatFmt::DEFAULT),
            fmt_f32(hi, FloatFmt::DEFAULT)
        ),
    }
}
