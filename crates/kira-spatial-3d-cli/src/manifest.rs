use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ManifestV1 {
    pub version: String,
    pub domain: ManifestDomain,
    pub field: ManifestField,
    pub mapping: ManifestMapping,
    pub contours: Option<ManifestContours>,
    pub export: ManifestExport,
}

#[derive(Debug, Deserialize)]
pub struct ManifestDomain {
    pub nx: usize,
    pub ny: usize,
    pub origin_x: f32,
    pub origin_y: f32,
    pub step_x: f32,
    pub step_y: f32,
}

#[derive(Debug, Deserialize)]
pub struct ManifestField {
    pub name: String,
    pub format: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct ManifestMapping {
    pub mode: ManifestHeightMode,
    pub normalization: ManifestNormalization,
    pub z_scale: f32,
    pub z_offset: f32,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum ManifestHeightMode {
    Raw,
    Abs,
    Signed,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(tag = "type")]
pub enum ManifestNormalization {
    None,
    MinMax { clip: Option<(f32, f32)> },
    RobustZ { clip_z: Option<(f32, f32)> },
    Percentile { lo: f32, hi: f32 },
}

#[derive(Debug, Deserialize)]
pub struct ManifestContours {
    pub levels: Vec<f32>,
    pub quantize_grid: f32,
}

#[derive(Debug, Deserialize)]
pub struct ManifestExport {
    pub out_dir: String,
    pub float_decimals: usize,
    pub write_obj: bool,
    pub write_ply: bool,
    pub write_polylines: bool,
    pub write_metrics: bool,
    pub write_metadata: bool,
}
