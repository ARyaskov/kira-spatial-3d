//! Deterministic projection from scalar fields to 3D mesh primitives.
//!
//! Stage 1 API exposes regular-grid core types and a deterministic heightmap mesh builder.

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
    ContourSegment, ContourSet, MultiContour, Polyline, PolylineSet, QKey, Quantize, StitchOptions,
    extract_contours, extract_ridge_contours, qkey, stitch_contours,
};
pub use export::{
    BufferOptions, ContourMeta, ContourMetaInput, DomainMeta, ExportBundleOptions, FloatFmt,
    HeightMeta, K3dMeshMeta, ObjOptions, PlyOptions, PolylineJson, PolylineJsonItem,
    RidgeMetricsJson, Spatial3dMetadata, TsvOptions, export_bundle, fmt_f32, save_metadata_json,
    save_obj, save_ply, save_polylines_json, save_polylines_tsv, save_ridge_metrics_json,
    save_ridge_metrics_tsv, write_k3d_mesh_buffer, write_metadata_json, write_obj, write_ply,
    write_polylines_json, write_polylines_tsv, write_ridge_metrics_json, write_ridge_metrics_tsv,
};
#[cfg(feature = "gltf")]
pub use export::{GltfOptions, write_gltf};
pub use mapping::{
    HeightMapSpec, HeightMode, Normalization, NormalizeOptions, build_heightmap_mesh_mapped,
    build_heights, normalize, validate_normalization,
};
pub use mesh::heightmap::{HeightmapOptions, build_heightmap_mesh};
pub use metrics::{RidgeMetrics, compute_ridge_metrics, ridges_to_polylines_and_metrics};
pub use types::{Error, Mesh, ScalarField, SpatialDomain};
