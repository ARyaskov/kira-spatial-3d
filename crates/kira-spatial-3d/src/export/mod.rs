pub mod buffer;
pub mod floatfmt;
#[cfg(feature = "gltf")]
pub mod gltf;
pub mod metadata;
pub mod obj;
pub mod ply;
pub mod polyline;

pub use buffer::{BufferOptions, K3dMeshMeta, write_k3d_mesh_buffer};
pub use floatfmt::{FloatFmt, fmt_f32};
#[cfg(feature = "gltf")]
pub use gltf::{GltfOptions, write_gltf};
pub use metadata::{
    ContourMeta, ContourMetaInput, DomainMeta, HeightMeta, Spatial3dMetadata, save_metadata_json,
    write_metadata_json,
};
pub use obj::{ObjOptions, save_obj, write_obj};
pub use ply::{PlyOptions, save_ply, write_ply};
pub use polyline::{
    PolylineJson, PolylineJsonItem, RidgeMetricsJson, TsvOptions, save_polylines_json,
    save_polylines_tsv, save_ridge_metrics_json, save_ridge_metrics_tsv, write_polylines_json,
    write_polylines_tsv, write_ridge_metrics_json, write_ridge_metrics_tsv,
};

use std::fs::create_dir_all;
use std::path::Path;

use crate::contour::PolylineSet;
use crate::metrics::RidgeMetrics;
use crate::{Error, Mesh};

/// Options for exporting a deterministic artifact bundle.
#[derive(Clone, Copy, Debug)]
pub struct ExportBundleOptions {
    pub float: FloatFmt,
    pub write_obj: bool,
    pub write_ply: bool,
    pub obj_normals: bool,
    pub ply_normals: bool,
    pub write_k3d: bool,
    #[cfg(feature = "gltf")]
    pub write_gltf: bool,
}

impl Default for ExportBundleOptions {
    fn default() -> Self {
        Self {
            float: FloatFmt::DEFAULT,
            write_obj: true,
            write_ply: true,
            obj_normals: true,
            ply_normals: true,
            write_k3d: false,
            #[cfg(feature = "gltf")]
            write_gltf: false,
        }
    }
}

/// Exports geometry artifacts into `out_dir` with deterministic file names.
pub fn export_bundle<P: AsRef<Path>>(
    out_dir: P,
    mesh: Option<&Mesh>,
    polylines: Option<&PolylineSet>,
    metrics: Option<&RidgeMetrics>,
    metadata: Option<&Spatial3dMetadata>,
    opts: ExportBundleOptions,
) -> Result<(), Error> {
    let out_dir = out_dir.as_ref();
    create_dir_all(out_dir)?;

    if let Some(mesh) = mesh {
        if opts.write_obj {
            save_obj(
                mesh,
                out_dir.join("surface.obj"),
                ObjOptions {
                    float: opts.float,
                    write_normals: opts.obj_normals,
                },
            )?;
        }
        if opts.write_ply {
            save_ply(
                mesh,
                out_dir.join("surface.ply"),
                PlyOptions {
                    float: opts.float,
                    write_normals: opts.ply_normals,
                },
            )?;
        }

        if opts.write_k3d {
            write_k3d_mesh_buffer(
                mesh,
                out_dir.join("mesh"),
                BufferOptions {
                    write_normals: opts.ply_normals,
                },
            )?;
        }

        #[cfg(feature = "gltf")]
        if opts.write_gltf {
            write_gltf(
                mesh,
                out_dir.join("mesh"),
                GltfOptions {
                    write_normals: opts.ply_normals,
                },
            )?;
        }
    }

    if let Some(polylines) = polylines {
        save_polylines_json(polylines, out_dir.join("polylines.json"))?;
        save_polylines_tsv(
            polylines,
            out_dir.join("polylines.tsv"),
            TsvOptions { float: opts.float },
        )?;
    }

    if let Some(metrics) = metrics {
        save_ridge_metrics_json(metrics, out_dir.join("ridge_metrics.json"))?;
        save_ridge_metrics_tsv(
            metrics,
            out_dir.join("ridge_metrics.tsv"),
            TsvOptions { float: opts.float },
        )?;
    }

    if let Some(metadata) = metadata {
        save_metadata_json(metadata, out_dir.join("metadata.json"))?;
    }

    Ok(())
}
