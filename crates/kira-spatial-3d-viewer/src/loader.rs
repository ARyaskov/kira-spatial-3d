use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use kira_spatial_3d::K3dMeshMeta;

use crate::ViewerError;

#[derive(Debug, Clone)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub indices: Vec<u32>,
    pub bbox: BoundingBox,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub center: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Deserialize)]
pub struct PolylineJson {
    pub level: f32,
    pub polylines: Vec<PolylineJsonItem>,
}

#[derive(Debug, Deserialize)]
pub struct PolylineJsonItem {
    pub is_closed: bool,
    pub points: Vec<[f32; 3]>,
}

#[derive(Debug, Clone)]
pub struct PolylineLayer {
    pub level: f32,
    pub points: Vec<[f32; 3]>,
    pub source: PathBuf,
}

pub fn load_mesh_prefix(prefix: &Path) -> Result<MeshData, ViewerError> {
    let json_path = with_ext(prefix, "k3d.json");
    let bin_path = with_ext(prefix, "k3d.bin");

    let meta: K3dMeshMeta = serde_json::from_slice(&fs::read(&json_path)?)?;
    validate_meta(&meta)?;

    let bytes = fs::read(&bin_path)?;
    validate_layout(&meta, bytes.len() as u64)?;

    let positions = decode_vec3_f32(&bytes, meta.positions_offset, meta.positions_bytes)?;
    let normals = if meta.normals_bytes > 0 {
        Some(decode_vec3_f32(
            &bytes,
            meta.normals_offset,
            meta.normals_bytes,
        )?)
    } else {
        None
    };
    let indices = decode_u32(&bytes, meta.indices_offset, meta.indices_bytes)?;

    if positions.len() != meta.vertex_count as usize {
        return Err(ViewerError::Data(
            "position count does not match metadata".to_string(),
        ));
    }
    if let Some(ns) = &normals
        && ns.len() != meta.vertex_count as usize
    {
        return Err(ViewerError::Data(
            "normal count does not match metadata".to_string(),
        ));
    }
    if indices.len() != meta.index_count as usize {
        return Err(ViewerError::Data(
            "index count does not match metadata".to_string(),
        ));
    }

    let bbox = compute_bounding_box(&positions);
    Ok(MeshData {
        positions,
        normals,
        indices,
        bbox,
    })
}

pub fn load_polyline_layer(path: &Path) -> Result<PolylineLayer, ViewerError> {
    let src: PolylineJson = serde_json::from_slice(&fs::read(path)?)?;
    let mut out = Vec::<[f32; 3]>::new();

    for p in &src.polylines {
        if p.points.len() < 2 {
            continue;
        }
        for w in p.points.windows(2) {
            out.push(sanitize_point(w[0]));
            out.push(sanitize_point(w[1]));
        }
        if p.is_closed {
            out.push(sanitize_point(*p.points.last().expect("len checked")));
            out.push(sanitize_point(p.points[0]));
        }
    }
    Ok(PolylineLayer {
        level: src.level,
        points: out,
        source: path.to_path_buf(),
    })
}

pub fn load_polyline_layers(path: &Path) -> Result<(Vec<PolylineLayer>, usize), ViewerError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut candidates = Vec::<PathBuf>::new();
    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(v) => v,
            None => continue,
        };
        if name.starts_with("polylines.level_") && name.ends_with(".json") {
            candidates.push(p);
        }
    }

    if candidates.is_empty() {
        let layer = load_polyline_layer(path)?;
        return Ok((vec![layer], 0));
    }

    let mut layers = Vec::<PolylineLayer>::with_capacity(candidates.len());
    for p in candidates {
        layers.push(load_polyline_layer(&p)?);
    }
    layers.sort_by(|a, b| a.level.total_cmp(&b.level));

    let mut active_idx = 0usize;
    if let Some(name) = path.file_name().and_then(|s| s.to_str())
        && let Some((idx, _)) = layers
            .iter()
            .enumerate()
            .find(|(_, l)| l.source.file_name().and_then(|s| s.to_str()) == Some(name))
    {
        active_idx = idx;
    }

    Ok((layers, active_idx))
}

pub fn compute_bounding_box(points: &[[f32; 3]]) -> BoundingBox {
    if points.is_empty() {
        return BoundingBox {
            min: [0.0, 0.0, 0.0],
            max: [0.0, 0.0, 0.0],
            center: [0.0, 0.0, 0.0],
            radius: 1.0,
        };
    }

    let mut min = sanitize_point(points[0]);
    let mut max = min;
    for &p in points.iter().skip(1) {
        let p = sanitize_point(p);
        for i in 0..3 {
            if p[i] < min[i] {
                min[i] = p[i];
            }
            if p[i] > max[i] {
                max[i] = p[i];
            }
        }
    }

    let center = [
        0.5 * (min[0] + max[0]),
        0.5 * (min[1] + max[1]),
        0.5 * (min[2] + max[2]),
    ];
    let dx = max[0] - center[0];
    let dy = max[1] - center[1];
    let dz = max[2] - center[2];
    let radius = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-4);

    BoundingBox {
        min,
        max,
        center,
        radius,
    }
}

fn validate_meta(meta: &K3dMeshMeta) -> Result<(), ViewerError> {
    if meta.version != "k3d-mesh-buffer/v1" {
        return Err(ViewerError::Data(format!(
            "unsupported k3d version: {}",
            meta.version
        )));
    }
    if meta.index_format != "u32" {
        return Err(ViewerError::Data(format!(
            "unsupported index format: {}",
            meta.index_format
        )));
    }
    Ok(())
}

fn validate_layout(meta: &K3dMeshMeta, len: u64) -> Result<(), ViewerError> {
    let pos_end = meta
        .positions_offset
        .checked_add(meta.positions_bytes)
        .ok_or_else(|| ViewerError::Data("positions range overflow".to_string()))?;
    let nor_end = meta
        .normals_offset
        .checked_add(meta.normals_bytes)
        .ok_or_else(|| ViewerError::Data("normals range overflow".to_string()))?;
    let idx_end = meta
        .indices_offset
        .checked_add(meta.indices_bytes)
        .ok_or_else(|| ViewerError::Data("indices range overflow".to_string()))?;

    if meta.positions_offset != 0 {
        return Err(ViewerError::Data("positions_offset must be 0".to_string()));
    }
    if meta.normals_offset != pos_end {
        return Err(ViewerError::Data(
            "normals_offset does not match positions end".to_string(),
        ));
    }
    if meta.indices_offset != nor_end {
        return Err(ViewerError::Data(
            "indices_offset does not match normals end".to_string(),
        ));
    }
    if idx_end != len {
        return Err(ViewerError::Data(format!(
            "k3d size mismatch: expected {idx_end}, got {len}"
        )));
    }
    if !meta.positions_bytes.is_multiple_of(12)
        || !meta.normals_bytes.is_multiple_of(12)
        || !meta.indices_bytes.is_multiple_of(4)
    {
        return Err(ViewerError::Data(
            "k3d block sizes are not aligned to element widths".to_string(),
        ));
    }

    Ok(())
}

fn decode_vec3_f32(bytes: &[u8], offset: u64, size: u64) -> Result<Vec<[f32; 3]>, ViewerError> {
    let start =
        usize::try_from(offset).map_err(|_| ViewerError::Data("offset too large".to_string()))?;
    let len = usize::try_from(size).map_err(|_| ViewerError::Data("size too large".to_string()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| ViewerError::Data("slice range overflow".to_string()))?;
    let s = bytes
        .get(start..end)
        .ok_or_else(|| ViewerError::Data("slice out of bounds".to_string()))?;

    let mut vals = Vec::<[f32; 3]>::with_capacity(s.len() / 12);
    for c in s.chunks_exact(12) {
        let x = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        let y = f32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        let z = f32::from_le_bytes([c[8], c[9], c[10], c[11]]);
        vals.push(sanitize_point([x, y, z]));
    }
    Ok(vals)
}

fn decode_u32(bytes: &[u8], offset: u64, size: u64) -> Result<Vec<u32>, ViewerError> {
    let start =
        usize::try_from(offset).map_err(|_| ViewerError::Data("offset too large".to_string()))?;
    let len = usize::try_from(size).map_err(|_| ViewerError::Data("size too large".to_string()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| ViewerError::Data("slice range overflow".to_string()))?;
    let s = bytes
        .get(start..end)
        .ok_or_else(|| ViewerError::Data("slice out of bounds".to_string()))?;

    let mut vals = Vec::<u32>::with_capacity(s.len() / 4);
    for c in s.chunks_exact(4) {
        vals.push(u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
    }
    Ok(vals)
}

fn with_ext(prefix: &Path, ext: &str) -> PathBuf {
    PathBuf::from(format!("{}.{}", prefix.display(), ext))
}

fn sanitize_point(mut p: [f32; 3]) -> [f32; 3] {
    for c in &mut p {
        if !c.is_finite() || *c == 0.0 {
            *c = 0.0;
        }
    }
    p
}
