#![cfg(feature = "gltf")]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::Error;
use crate::Mesh;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GltfOptions {
    pub write_normals: bool,
}

impl Default for GltfOptions {
    fn default() -> Self {
        Self {
            write_normals: true,
        }
    }
}

pub fn write_gltf<P: AsRef<Path>>(
    mesh: &Mesh,
    out_prefix: P,
    opts: GltfOptions,
) -> Result<(PathBuf, PathBuf), Error> {
    let out_prefix = out_prefix.as_ref();
    if let Some(parent) = out_prefix.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bin_path = PathBuf::from(format!("{}.bin", out_prefix.display()));
    let gltf_path = PathBuf::from(format!("{}.gltf", out_prefix.display()));

    let positions_bytes = mesh.vertices.len() as u32 * 3 * 4;
    let normals_bytes = if opts.write_normals {
        mesh.normals.len() as u32 * 3 * 4
    } else {
        0
    };
    let indices_bytes = mesh.indices.len() as u32 * 4;

    let positions_offset = 0_u32;
    let normals_offset = positions_offset + positions_bytes;
    let indices_offset = normals_offset + normals_bytes;
    let total_bytes = indices_offset + indices_bytes;

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

    let (pos_min, pos_max) = position_min_max(&mesh.vertices);

    let mut buffer_views = vec![BufferView {
        buffer: 0,
        byte_offset: positions_offset,
        byte_length: positions_bytes,
        target: Some(34962),
    }];
    let mut accessors = vec![Accessor {
        buffer_view: 0,
        byte_offset: 0,
        component_type: 5126,
        count: mesh.vertices.len() as u32,
        accessor_type: "VEC3",
        min: Some(vec![pos_min[0], pos_min[1], pos_min[2]]),
        max: Some(vec![pos_max[0], pos_max[1], pos_max[2]]),
    }];

    let normal_accessor = if opts.write_normals {
        let view_idx = buffer_views.len() as u32;
        buffer_views.push(BufferView {
            buffer: 0,
            byte_offset: normals_offset,
            byte_length: normals_bytes,
            target: Some(34962),
        });
        let accessor_idx = accessors.len() as u32;
        accessors.push(Accessor {
            buffer_view: view_idx,
            byte_offset: 0,
            component_type: 5126,
            count: mesh.normals.len() as u32,
            accessor_type: "VEC3",
            min: None,
            max: None,
        });
        Some(accessor_idx)
    } else {
        None
    };

    let index_view = buffer_views.len() as u32;
    buffer_views.push(BufferView {
        buffer: 0,
        byte_offset: indices_offset,
        byte_length: indices_bytes,
        target: Some(34963),
    });
    let index_accessor = accessors.len() as u32;
    accessors.push(Accessor {
        buffer_view: index_view,
        byte_offset: 0,
        component_type: 5125,
        count: mesh.indices.len() as u32,
        accessor_type: "SCALAR",
        min: None,
        max: None,
    });

    let bin_uri = bin_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or(Error::InvalidExportSpec {
            message: "invalid gltf bin filename",
        })?;

    let gltf = GltfRoot {
        asset: Asset {
            version: "2.0",
            generator: "kira-spatial-3d",
        },
        scene: 0,
        scenes: vec![Scene { nodes: vec![0] }],
        nodes: vec![Node { mesh: 0 }],
        meshes: vec![GltfMesh {
            primitives: vec![Primitive {
                attributes: PrimitiveAttributes {
                    position: 0,
                    normal: normal_accessor,
                },
                indices: index_accessor,
                mode: 4,
            }],
        }],
        buffers: vec![Buffer {
            uri: bin_uri,
            byte_length: total_bytes,
        }],
        buffer_views,
        accessors,
    };

    {
        let file = File::create(&gltf_path)?;
        let w = BufWriter::new(file);
        serde_json::to_writer(w, &gltf)?;
    }

    Ok((gltf_path, bin_path))
}

fn position_min_max(vertices: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    if vertices.is_empty() {
        return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    }

    let first = sanitize(vertices[0]);
    let mut min = first;
    let mut max = first;
    for &p in vertices.iter().skip(1) {
        let p = sanitize(p);
        for i in 0..3 {
            if p[i] < min[i] {
                min[i] = p[i];
            }
            if p[i] > max[i] {
                max[i] = p[i];
            }
        }
    }
    (min, max)
}

fn sanitize(mut p: [f32; 3]) -> [f32; 3] {
    for c in &mut p {
        if !c.is_finite() || *c == 0.0 {
            *c = 0.0;
        }
    }
    p
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GltfRoot {
    asset: Asset,
    scene: u32,
    scenes: Vec<Scene>,
    nodes: Vec<Node>,
    meshes: Vec<GltfMesh>,
    buffers: Vec<Buffer>,
    buffer_views: Vec<BufferView>,
    accessors: Vec<Accessor>,
}

#[derive(Serialize)]
struct Asset {
    version: &'static str,
    generator: &'static str,
}

#[derive(Serialize)]
struct Scene {
    nodes: Vec<u32>,
}

#[derive(Serialize)]
struct Node {
    mesh: u32,
}

#[derive(Serialize)]
struct GltfMesh {
    primitives: Vec<Primitive>,
}

#[derive(Serialize)]
struct Primitive {
    attributes: PrimitiveAttributes,
    indices: u32,
    mode: u32,
}

#[derive(Serialize)]
struct PrimitiveAttributes {
    #[serde(rename = "POSITION")]
    position: u32,
    #[serde(rename = "NORMAL", skip_serializing_if = "Option::is_none")]
    normal: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Buffer {
    uri: String,
    byte_length: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BufferView {
    buffer: u32,
    byte_offset: u32,
    byte_length: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Accessor {
    buffer_view: u32,
    byte_offset: u32,
    component_type: u32,
    count: u32,
    #[serde(rename = "type")]
    accessor_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<Vec<f32>>,
}
