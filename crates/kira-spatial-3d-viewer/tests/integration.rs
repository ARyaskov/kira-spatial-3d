use std::fs::File;
use std::io::Write;

use kira_spatial_3d::K3dMeshMeta;
use kira_spatial_3d_viewer::loader::{compute_bounding_box, load_mesh_prefix};
use tempfile::tempdir;

#[test]
fn parses_k3d_metadata_and_binary() {
    let dir = tempdir().expect("tmp");
    let prefix = dir.path().join("mesh");
    let json_path = dir.path().join("mesh.k3d.json");
    let bin_path = dir.path().join("mesh.k3d.bin");

    let positions = vec![[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let normals = vec![[0.0_f32, 0.0, 1.0]; 3];
    let indices = vec![0_u32, 1, 2];

    let mut bin = Vec::<u8>::new();
    for p in &positions {
        for c in p {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    for n in &normals {
        for c in n {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    for i in &indices {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    File::create(&bin_path)
        .expect("bin create")
        .write_all(&bin)
        .expect("bin write");

    let meta = K3dMeshMeta {
        version: "k3d-mesh-buffer/v1".to_string(),
        vertex_count: 3,
        index_count: 3,
        positions_offset: 0,
        normals_offset: 36,
        indices_offset: 72,
        positions_bytes: 36,
        normals_bytes: 36,
        indices_bytes: 12,
        index_format: "u32".to_string(),
    };
    serde_json::to_writer(File::create(&json_path).expect("json create"), &meta)
        .expect("json write");

    let mesh = load_mesh_prefix(&prefix).expect("load");
    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.normals.as_ref().expect("normals").len(), 3);
    assert_eq!(mesh.indices, indices);
}

#[test]
fn bounding_box_is_correct() {
    let points = vec![[0.0, 0.0, 0.0], [2.0, -1.0, 3.0], [1.0, 1.0, -2.0]];
    let b = compute_bounding_box(&points);

    assert_eq!(b.min, [0.0, -1.0, -2.0]);
    assert_eq!(b.max, [2.0, 1.0, 3.0]);
    assert_eq!(b.center, [1.0, 0.0, 0.5]);
    assert!(b.radius.is_finite());
    assert!(b.radius > 0.0);
}
