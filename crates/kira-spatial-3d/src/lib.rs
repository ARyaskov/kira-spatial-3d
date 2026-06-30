#![deny(unsafe_code)]

//! Deterministic projection from scalar fields to 3D mesh primitives.

pub mod config;
pub mod contour;
pub mod export;
pub mod mapping;
pub mod mesh;
pub mod metrics;
mod simd;
pub mod types;

pub use config::{ComputeBackend, ComputeConfig};
pub use contour::{
    ContourSegment, ContourSet, ContourStats, MultiContour, Polyline, PolylineSet, QKey, Quantize,
    StitchOptions, extract_contours, extract_contours_with_stats, extract_ridge_contours,
    extract_ridge_polylines, for_each_contour_segment, qkey, stitch_contours,
};
pub use export::{
    BufferOptions, ContourMeta, ContourMetaInput, DomainMeta, ExportBundleOptions, FloatFmt,
    HeightMeta, K3dMeshMeta, ObjOptions, PlyOptions, PolylineJson, PolylineJsonItem,
    RidgeMetricsJson, Spatial3dMetadata, TsvOptions, export_bundle, fmt_f32, height_mode_name,
    normalization_name, save_metadata_json, save_obj, save_ply, save_polylines_json,
    save_polylines_tsv, save_ridge_metrics_json, save_ridge_metrics_tsv, write_fmt_f32,
    write_k3d_mesh_buffer, write_metadata_json, write_obj, write_ply, write_polylines_json,
    write_polylines_tsv, write_ridge_metrics_json, write_ridge_metrics_tsv,
};
#[cfg(feature = "gltf")]
pub use export::{GltfOptions, write_gltf};
pub use mapping::{
    HeightMapSpec, HeightMode, Normalization, NormalizeOptions, build_heightmap_mesh_mapped,
    build_heights, normalize, normalize_with, validate_normalization,
};
pub use mesh::heightmap::{HeightmapOptions, build_heightmap_mesh};
pub use mesh::streaming::build_heightmap_mesh_to_k3d;
pub use metrics::{RidgeMetrics, compute_ridge_metrics, ridges_to_polylines_and_metrics};
#[cfg(feature = "with-field")]
pub use types::from_kira_field;
pub use types::{Error, Mesh, MeshBounds, OwnedScalarField, ScalarField, SpatialDomain};
