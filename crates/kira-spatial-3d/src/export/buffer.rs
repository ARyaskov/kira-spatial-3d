use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Mesh};

/// Metadata for deterministic K3D mesh buffer v1.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct K3dMeshMeta {
    pub version: String,
    pub vertex_count: u32,
    pub index_count: u32,
    pub positions_offset: u64,
    pub normals_offset: u64,
    pub indices_offset: u64,
    pub positions_bytes: u64,
    pub normals_bytes: u64,
    pub indices_bytes: u64,
    pub index_format: String,
}

/// Options for K3D buffer export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferOptions {
    pub write_normals: bool,
}

impl Default for BufferOptions {
    fn default() -> Self {
        Self {
            write_normals: true,
        }
    }
}

/// Writes `<prefix>.k3d.bin` and `<prefix>.k3d.json`.
pub fn write_k3d_mesh_buffer<P: AsRef<Path>>(
    mesh: &Mesh,
    out_prefix: P,
    opts: BufferOptions,
) -> Result<(PathBuf, PathBuf), Error> {
    let out_prefix = out_prefix.as_ref();
    if let Some(parent) = out_prefix.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bin_path = PathBuf::from(format!("{}.k3d.bin", out_prefix.display()));
    let json_path = PathBuf::from(format!("{}.k3d.json", out_prefix.display()));

    let vertex_count =
        u32::try_from(mesh.vertices.len()).map_err(|_| Error::InvalidExportSpec {
            message: "vertex_count exceeds u32",
        })?;
    let index_count = u32::try_from(mesh.indices.len()).map_err(|_| Error::InvalidExportSpec {
        message: "index_count exceeds u32",
    })?;

    let positions_bytes = mesh.vertices.len() as u64 * 3 * 4;
    let normals_bytes = if opts.write_normals {
        mesh.normals.len() as u64 * 3 * 4
    } else {
        0
    };
    let indices_bytes = mesh.indices.len() as u64 * 4;

    let positions_offset = 0_u64;
    let normals_offset = positions_offset + positions_bytes;
    let indices_offset = normals_offset + normals_bytes;

    let meta = K3dMeshMeta {
        version: "k3d-mesh-buffer/v1".to_string(),
        vertex_count,
        index_count,
        positions_offset,
        normals_offset,
        indices_offset,
        positions_bytes,
        normals_bytes,
        indices_bytes,
        index_format: "u32".to_string(),
    };

    {
        let file = File::create(&bin_path)?;
        let mut w = BufWriter::new(file);

        for p in &mesh.vertices {
            for c in p {
                w.write_all(&c.to_le_bytes())?;
            }
        }

        if opts.write_normals {
            for n in &mesh.normals {
                for c in n {
                    w.write_all(&c.to_le_bytes())?;
                }
            }
        }

        for &idx in &mesh.indices {
            w.write_all(&idx.to_le_bytes())?;
        }
        w.flush()?;
    }

    {
        let file = File::create(&json_path)?;
        let w = BufWriter::new(file);
        serde_json::to_writer(w, &meta)?;
    }

    Ok((bin_path, json_path))
}
