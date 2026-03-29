use std::fmt;
use std::fs::{File, create_dir_all};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use kira_spatial_3d::{
    ComputeBackend, ComputeConfig, ContourMetaInput, FloatFmt, HeightMapSpec, HeightMode,
    HeightmapOptions, Normalization, RidgeMetrics, ScalarField, Spatial3dMetadata, SpatialDomain,
    StitchOptions, TsvOptions, build_heightmap_mesh, build_heights, compute_ridge_metrics,
    extract_contours, normalize, save_metadata_json, save_obj, save_ply, save_polylines_json,
    save_polylines_tsv, save_ridge_metrics_json, save_ridge_metrics_tsv, stitch_contours,
    validate_normalization,
};
use serde::Serialize;

pub mod manifest;

use manifest::{
    ManifestContours, ManifestDomain, ManifestHeightMode, ManifestNormalization, ManifestV1,
};

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Lib(kira_spatial_3d::Error),
    Manifest(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Lib(e) => write!(f, "library error: {e}"),
            Self::Manifest(e) => write!(f, "manifest error: {e}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<kira_spatial_3d::Error> for CliError {
    fn from(value: kira_spatial_3d::Error) -> Self {
        Self::Lib(value)
    }
}

pub fn run_manifest(
    manifest_path: &Path,
    force_scalar: bool,
    no_contours: bool,
) -> Result<(), CliError> {
    let manifest = read_manifest(manifest_path)?;
    validate_manifest_header(&manifest)?;

    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let field_path = base_dir.join(&manifest.field.path);
    if !field_path.exists() {
        return Err(CliError::Manifest(format!(
            "field file does not exist: {}",
            manifest.field.path
        )));
    }
    if manifest.field.format != "f32le" {
        return Err(CliError::Manifest(format!(
            "unsupported field format: {}",
            manifest.field.format
        )));
    }

    let domain = build_domain(&manifest.domain)?;
    let values = read_f32le(&field_path)?;
    if values.len() != domain.len() {
        return Err(CliError::Manifest(format!(
            "field length mismatch: expected {}, got {}",
            domain.len(),
            values.len()
        )));
    }

    let compute = if force_scalar {
        ComputeConfig {
            backend: ComputeBackend::Scalar,
        }
    } else {
        ComputeConfig::default()
    };

    let field = ScalarField::new(domain, &values)?;
    let mapping_norm = map_normalization(manifest.mapping.normalization);
    validate_normalization(&mapping_norm)?;

    let height_spec = HeightMapSpec {
        mode: map_height_mode(manifest.mapping.mode),
        normalization: mapping_norm,
        z_scale: manifest.mapping.z_scale,
        z_offset: manifest.mapping.z_offset,
        compute,
    };

    let (normed, z) = build_normed_and_z(&field, height_spec)?;

    let z_field = ScalarField::new(domain, &z)?;
    let mesh = build_heightmap_mesh(
        &z_field,
        HeightmapOptions {
            z_scale: 1.0,
            z_offset: 0.0,
            compute,
        },
    )?;

    let out_dir = base_dir.join(&manifest.export.out_dir);
    create_dir_all(&out_dir)?;

    let float = FloatFmt {
        decimals: manifest.export.float_decimals,
    };

    if manifest.export.write_obj {
        save_obj(
            &mesh,
            out_dir.join("surface.obj"),
            kira_spatial_3d::ObjOptions {
                float,
                write_normals: true,
            },
        )?;
    }
    if manifest.export.write_ply {
        save_ply(
            &mesh,
            out_dir.join("surface.ply"),
            kira_spatial_3d::PlyOptions {
                float,
                write_normals: true,
            },
        )?;
    }

    let contour_cfg = if no_contours {
        None
    } else {
        manifest.contours.as_ref()
    };
    let mut level_files = Vec::<String>::new();

    if let Some(contours) = contour_cfg {
        validate_contours(contours)?;

        let normed_field = ScalarField::new(domain, &normed)?;
        let multi = extract_contours(&normed_field, &contours.levels)?;
        for set in multi.contours {
            let poly = stitch_contours(
                &set,
                StitchOptions {
                    quantize: kira_spatial_3d::Quantize {
                        grid: contours.quantize_grid,
                    },
                },
            )?;
            let metrics: RidgeMetrics = compute_ridge_metrics(&poly);

            let level_tag = kira_spatial_3d::fmt_f32(set.level, float);
            if manifest.export.write_polylines {
                let json_name = format!("polylines.level_{level_tag}.json");
                let tsv_name = format!("polylines.level_{level_tag}.tsv");
                save_polylines_json(&poly, out_dir.join(&json_name))?;
                save_polylines_tsv(&poly, out_dir.join(&tsv_name), TsvOptions { float })?;
                level_files.push(json_name);
                level_files.push(tsv_name);
            }

            if manifest.export.write_metrics {
                let json_name = format!("ridge_metrics.level_{level_tag}.json");
                let tsv_name = format!("ridge_metrics.level_{level_tag}.tsv");
                save_ridge_metrics_json(&metrics, out_dir.join(&json_name))?;
                save_ridge_metrics_tsv(&metrics, out_dir.join(&tsv_name), TsvOptions { float })?;
                level_files.push(json_name);
                level_files.push(tsv_name);
            }
        }

        if contours.levels.len() > 1 && !level_files.is_empty() {
            let index = IndexJson {
                version: "kira-spatial-3d-cli/index/v1",
                files: level_files,
            };
            let f = File::create(out_dir.join("index.json"))?;
            serde_json::to_writer(f, &index)?;
        }
    }

    if manifest.export.write_metadata {
        let contour_meta = contour_cfg.map(|c| ContourMetaInput {
            levels: &c.levels,
            quantize: kira_spatial_3d::Quantize {
                grid: c.quantize_grid,
            },
        });
        let meta = Spatial3dMetadata::from_specs(domain, height_spec, contour_meta);
        save_metadata_json(&meta, out_dir.join("metadata.json"))?;
    }

    Ok(())
}

fn read_manifest(path: &Path) -> Result<ManifestV1, CliError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn validate_manifest_header(m: &ManifestV1) -> Result<(), CliError> {
    if m.version != "kira-spatial-manifest/v1" {
        return Err(CliError::Manifest(format!(
            "unsupported manifest version: {}",
            m.version
        )));
    }
    Ok(())
}

fn build_domain(d: &ManifestDomain) -> Result<SpatialDomain, CliError> {
    Ok(SpatialDomain::new(
        d.nx, d.ny, d.origin_x, d.origin_y, d.step_x, d.step_y,
    )?)
}

fn validate_contours(c: &ManifestContours) -> Result<(), CliError> {
    if c.levels.is_empty() {
        return Err(CliError::Manifest(
            "contours.levels must not be empty".to_string(),
        ));
    }
    if !c.quantize_grid.is_finite() || c.quantize_grid <= 0.0 {
        return Err(CliError::Manifest(
            "contours.quantize_grid must be finite and > 0".to_string(),
        ));
    }
    if c.levels.iter().any(|v| !v.is_finite()) {
        return Err(CliError::Manifest(
            "contours.levels must be finite".to_string(),
        ));
    }
    Ok(())
}

fn map_height_mode(mode: ManifestHeightMode) -> HeightMode {
    match mode {
        ManifestHeightMode::Raw => HeightMode::Raw,
        ManifestHeightMode::Abs => HeightMode::Abs,
        ManifestHeightMode::Signed => HeightMode::Signed,
    }
}

fn map_normalization(norm: ManifestNormalization) -> Normalization {
    match norm {
        ManifestNormalization::None => Normalization::None,
        ManifestNormalization::MinMax { clip } => Normalization::MinMax { clip },
        ManifestNormalization::RobustZ { clip_z } => Normalization::RobustZ { clip_z },
        ManifestNormalization::Percentile { lo, hi } => Normalization::Percentile { lo, hi },
    }
}

fn build_normed_and_z(
    field: &ScalarField<'_>,
    spec: HeightMapSpec,
) -> Result<(Vec<f32>, Vec<f32>), CliError> {
    let tmp = field
        .values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return f32::NAN;
            }
            match spec.mode {
                HeightMode::Raw | HeightMode::Signed => v,
                HeightMode::Abs => v.abs(),
            }
        })
        .collect::<Vec<f32>>();

    let normed = normalize(
        &tmp,
        kira_spatial_3d::NormalizeOptions {
            policy: spec.normalization,
        },
    );
    let z = build_heights(field, spec)?;
    Ok((normed, z))
}

fn read_f32le(path: &Path) -> Result<Vec<f32>, CliError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    if bytes.len() % 4 != 0 {
        return Err(CliError::Manifest(format!(
            "f32le byte length must be multiple of 4, got {}",
            bytes.len()
        )));
    }

    let mut values = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(values)
}

#[derive(Serialize)]
struct IndexJson {
    version: &'static str,
    files: Vec<String>,
}

pub fn run_manifest_path(
    path: PathBuf,
    force_scalar: bool,
    no_contours: bool,
) -> Result<(), CliError> {
    run_manifest(&path, force_scalar, no_contours)
}
